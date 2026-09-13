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

//! Call an ONDEWO SURVEY server with a Keycloak bearer token.
//!
//! This is the crate's usage snippet. It lives here rather than in a doc comment because doctests
//! are disabled crate-wide (see the `doctest = false` note in `Cargo.toml`); as an example it is
//! still compiled by `cargo test` and `cargo build --examples`, so it cannot rot.
//!
//! ```sh
//! ONDEWO_SURVEY_HOST=https://survey.example.com:443 \
//! ONDEWO_SURVEY_ACCESS_TOKEN=<keycloak access token> \
//! ONDEWO_SURVEY_CAI_TOKEN=<cai token> \
//! ONDEWO_SURVEY_ID=<survey id> \
//!   cargo run --example authenticated_client
//! ```

use std::env;
use std::error::Error;

use ondewo_survey_client::api::ondewo::survey;
use ondewo_survey_client::api::ondewo::survey::fhir_client::FhirClient;
use ondewo_survey_client::auth::BearerTokenInterceptor;
use tonic::transport::Endpoint;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let host = env::var("ONDEWO_SURVEY_HOST")?;
    let access_token = env::var("ONDEWO_SURVEY_ACCESS_TOKEN")?;

    let mut interceptor = BearerTokenInterceptor::new(&access_token)?;
    if let Ok(cai_token) = env::var("ONDEWO_SURVEY_CAI_TOKEN") {
        interceptor = interceptor.with_cai_token(&cai_token)?;
    }

    let channel = Endpoint::from_shared(host)?.connect().await?;
    let mut client = FhirClient::with_interceptor(channel, interceptor);

    let response = client
        .get_all_fhir_survey_answers(survey::GetAllSurveyAnswersRequest {
            survey_id: env::var("ONDEWO_SURVEY_ID")?,
        })
        .await?
        .into_inner();

    println!(
        "survey {} answered {} time(s)",
        response.survey_id,
        response.fhir_questionnaire_responses.len()
    );
    Ok(())
}
