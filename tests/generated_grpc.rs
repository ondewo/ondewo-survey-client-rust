// Copyright 2021-2026 ONDEWO GmbH
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! End-to-end tests for the GENERATED tonic service stubs.
//!
//! `ondewo-survey-api` declares two services, `Surveys` and `FHIR`, and both are entirely unary.
//! The generated `FhirServer` - the smaller of the two, so the fake implements a complete service
//! rather than a slice of one - is served over a loopback socket and driven by the generated
//! `FhirClient`, so a request really is encoded, routed by its `/ondewo.survey.FHIR/<Method>`
//! path, decoded, answered and decoded again. That is what catches a service the generator wired
//! to the wrong path, a codec mismatch, or a method that silently went missing.
//!
//! No network beyond `127.0.0.1` and no ONDEWO server is involved.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ondewo_survey_client::api::ondewo::survey;
use ondewo_survey_client::api::ondewo::survey::fhir_client::FhirClient;
use ondewo_survey_client::api::ondewo::survey::fhir_server::{Fhir, FhirServer};
use ondewo_survey_client::auth::{
    BearerTokenInterceptor, AUTHORIZATION_METADATA_KEY, CAI_TOKEN_METADATA_KEY,
};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::{Channel, Endpoint, Server};
use tonic::{Code, Request, Response, Status};

/// The survey id the answer RPCs answer with `not_found` for, so the error path is exercised too.
const MISSING_SURVEY: &str = "does-not-exist";

/// A `google.protobuf.Struct` holding a single string entry, the shape a FHIR questionnaire
/// travels in.
fn fhir_document(resource_type: &str) -> prost_types::Struct {
    let mut fields = BTreeMap::new();
    fields.insert(
        "resourceType".to_string(),
        prost_types::Value {
            kind: Some(prost_types::value::Kind::StringValue(
                resource_type.to_string(),
            )),
        },
    );
    prost_types::Struct { fields }
}

/// Metadata the fake server captured from the last request it handled.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct SeenMetadata {
    authorization: Option<String>,
    cai_token: Option<String>,
}

/// A minimal in-process implementation of the generated `FHIR` service.
#[derive(Clone, Default)]
struct FakeFhir {
    seen: Arc<Mutex<SeenMetadata>>,
}

impl FakeFhir {
    fn record<T>(&self, request: &Request<T>) {
        let read = |key: &str| {
            request
                .metadata()
                .get(key)
                .map(|value| value.to_str().unwrap().to_string())
        };
        *self.seen.lock().unwrap() = SeenMetadata {
            authorization: read(AUTHORIZATION_METADATA_KEY),
            cai_token: read(CAI_TOKEN_METADATA_KEY),
        };
    }

    fn seen(&self) -> SeenMetadata {
        self.seen.lock().unwrap().clone()
    }
}

#[tonic::async_trait]
impl Fhir for FakeFhir {
    /// Turns the questionnaire into a one-question survey, so the `google.protobuf.Struct` really
    /// has to survive the hop in order for the answer to be right.
    async fn create_fhir_survey(
        &self,
        request: Request<survey::CreateFhirSurveyRequest>,
    ) -> Result<Response<survey::Survey>, Status> {
        self.record(&request);
        let questionnaire = request
            .into_inner()
            .fhir_questionnaire
            .ok_or_else(|| Status::invalid_argument("fhir_questionnaire is required"))?;
        let resource_type = match questionnaire.fields.get("resourceType").and_then(|value| {
            value.kind.as_ref().map(|kind| match kind {
                prost_types::value::Kind::StringValue(text) => text.clone(),
                _ => String::new(),
            })
        }) {
            Some(resource_type) => resource_type,
            None => return Err(Status::invalid_argument("resourceType is required")),
        };
        Ok(Response::new(survey::Survey {
            survey_id: format!("survey-from-{resource_type}"),
            display_name: resource_type,
            questions: vec![survey::Question {
                question: Some(survey::question::Question::OpenQuestion(
                    survey::OpenQuestion {
                        question_text: "How did we do?".to_string(),
                    },
                )),
            }],
            status: survey::survey::AgentStatus::Updated as i32,
            ..Default::default()
        }))
    }

    async fn get_fhir_survey_answers(
        &self,
        request: Request<survey::GetSurveyAnswersRequest>,
    ) -> Result<Response<survey::SurveyFhirAnswersResponse>, Status> {
        self.record(&request);
        let request = request.into_inner();
        if request.survey_id == MISSING_SURVEY {
            return Err(Status::not_found(format!(
                "no survey named {}",
                request.survey_id
            )));
        }
        Ok(Response::new(survey::SurveyFhirAnswersResponse {
            survey_id: request.survey_id,
            fhir_questionnaire_responses: vec![fhir_document("QuestionnaireResponse")],
        }))
    }

    async fn get_all_fhir_survey_answers(
        &self,
        request: Request<survey::GetAllSurveyAnswersRequest>,
    ) -> Result<Response<survey::SurveyFhirAnswersResponse>, Status> {
        self.record(&request);
        let request = request.into_inner();
        if request.survey_id == MISSING_SURVEY {
            return Err(Status::not_found(format!(
                "no survey named {}",
                request.survey_id
            )));
        }
        Ok(Response::new(survey::SurveyFhirAnswersResponse {
            survey_id: request.survey_id,
            fhir_questionnaire_responses: vec![
                fhir_document("QuestionnaireResponse"),
                fhir_document("QuestionnaireResponse"),
            ],
        }))
    }
}

/// Start the generated server on an ephemeral loopback port and return it with its address.
///
/// The server task is detached; it ends when the test process does.
async fn start_server() -> (FakeFhir, SocketAddr) {
    let service = FakeFhir::default();
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");

    let served = service.clone();
    tokio::spawn(async move {
        Server::builder()
            .add_service(FhirServer::new(served))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .expect("the in-process gRPC server must not fail");
    });

    (service, addr)
}

async fn connect(addr: SocketAddr) -> Channel {
    Endpoint::from_shared(format!("http://{addr}"))
        .expect("endpoint")
        .connect_timeout(Duration::from_secs(10))
        .connect()
        .await
        .expect("the in-process gRPC server must accept a connection")
}

#[tokio::test]
async fn a_unary_call_round_trips_through_the_generated_client_and_server() {
    let (_service, addr) = start_server().await;
    let mut client = FhirClient::new(connect(addr).await);

    let response = client
        .get_all_fhir_survey_answers(survey::GetAllSurveyAnswersRequest {
            survey_id: "survey-42".to_string(),
        })
        .await
        .expect("GetAllFHIRSurveyAnswers must succeed")
        .into_inner();

    assert_eq!(response.survey_id, "survey-42");
    assert_eq!(response.fhir_questionnaire_responses.len(), 2);
    assert_eq!(
        response.fhir_questionnaire_responses[0],
        fhir_document("QuestionnaireResponse")
    );
}

/// A `google.protobuf.Struct` is the well-known type this API carries its FHIR documents in, so
/// it has to survive the hop with its entries intact rather than arrive empty.
#[tokio::test]
async fn a_well_known_struct_survives_a_real_grpc_hop() {
    let (_service, addr) = start_server().await;
    let mut client = FhirClient::new(connect(addr).await);

    let created = client
        .create_fhir_survey(survey::CreateFhirSurveyRequest {
            fhir_questionnaire: Some(fhir_document("Questionnaire")),
        })
        .await
        .expect("CreateFHIRSurvey must succeed")
        .into_inner();

    assert_eq!(created.survey_id, "survey-from-Questionnaire");
    assert_eq!(created.display_name, "Questionnaire");
    assert_eq!(created.questions.len(), 1);
    assert_eq!(
        created.status,
        survey::survey::AgentStatus::Updated as i32,
        "a non-zero enum must not come back as the zero value"
    );
}

/// A server-side `Status` has to reach the caller as that same status, not as a transport error.
#[tokio::test]
async fn a_server_error_reaches_the_client_as_its_status() {
    let (_service, addr) = start_server().await;
    let mut client = FhirClient::new(connect(addr).await);

    let error = client
        .get_fhir_survey_answers(survey::GetSurveyAnswersRequest {
            survey_id: MISSING_SURVEY.to_string(),
            identifier: None,
        })
        .await
        .expect_err("GetFHIRSurveyAnswers must report the missing survey");

    assert_eq!(error.code(), Code::NotFound);
    assert_eq!(error.message(), "no survey named does-not-exist");
}

/// Every RPC the `FHIR` proto declares must exist on the generated client and be routable -
/// a method the generator dropped, or wired to the wrong path, fails here with `Unimplemented`.
#[tokio::test]
async fn every_declared_service_method_exists_and_is_routable() {
    let (_service, addr) = start_server().await;
    let mut client = FhirClient::new(connect(addr).await);

    client
        .create_fhir_survey(survey::CreateFhirSurveyRequest {
            fhir_questionnaire: Some(fhir_document("Questionnaire")),
        })
        .await
        .expect("CreateFHIRSurvey");
    client
        .get_fhir_survey_answers(survey::GetSurveyAnswersRequest {
            survey_id: "survey-42".to_string(),
            identifier: Some(survey::get_survey_answers_request::Identifier::SessionId(
                "session-1".to_string(),
            )),
        })
        .await
        .expect("GetFHIRSurveyAnswers");
    client
        .get_all_fhir_survey_answers(survey::GetAllSurveyAnswersRequest {
            survey_id: "survey-42".to_string(),
        })
        .await
        .expect("GetAllFHIRSurveyAnswers");
}

/// The hand-written [`BearerTokenInterceptor`] has to put its metadata on the wire, where the
/// server can actually read it - asserting on the `Request` it returns would not prove that.
#[tokio::test]
async fn the_bearer_interceptor_reaches_the_server() {
    let (service, addr) = start_server().await;
    let interceptor = BearerTokenInterceptor::new("access-token-abc")
        .expect("a plain ASCII token is valid")
        .with_cai_token("cai-token-xyz")
        .expect("a plain ASCII cai token is valid");
    let mut client = FhirClient::with_interceptor(connect(addr).await, interceptor);

    client
        .get_all_fhir_survey_answers(survey::GetAllSurveyAnswersRequest {
            survey_id: "survey-42".to_string(),
        })
        .await
        .expect("GetAllFHIRSurveyAnswers");

    assert_eq!(
        service.seen(),
        SeenMetadata {
            authorization: Some("Bearer access-token-abc".to_string()),
            cai_token: Some("cai-token-xyz".to_string()),
        }
    );
}

/// Without the interceptor the client must send no credentials at all - the unauthenticated path
/// (plaintext server, or an ingress that injects the bearer token) has to stay usable.
#[tokio::test]
async fn a_client_without_an_interceptor_sends_no_credentials() {
    let (service, addr) = start_server().await;
    let mut client = FhirClient::new(connect(addr).await);

    client
        .get_all_fhir_survey_answers(survey::GetAllSurveyAnswersRequest {
            survey_id: "survey-42".to_string(),
        })
        .await
        .expect("GetAllFHIRSurveyAnswers");

    assert_eq!(service.seen(), SeenMetadata::default());
}

/// A client built against an address nothing listens on must surface a transport error rather
/// than panic or hang - `connect_lazy` defers the connect to the first call.
#[tokio::test]
async fn a_call_to_an_unreachable_target_fails_as_a_status() {
    let channel = Endpoint::from_static("http://127.0.0.1:1")
        .connect_timeout(Duration::from_secs(2))
        .connect_lazy();
    let mut client = FhirClient::new(channel);

    let error = client
        .get_all_fhir_survey_answers(survey::GetAllSurveyAnswersRequest {
            survey_id: "survey-42".to_string(),
        })
        .await
        .expect_err("nothing listens on port 1");

    assert_eq!(error.code(), Code::Unavailable);
}
