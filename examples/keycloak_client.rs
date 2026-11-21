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

//! Call an ONDEWO SURVEY server with an auto-refreshed Keycloak token.
//!
//! With `ONDEWO_SURVEY_OFFLINE_TOKEN` set, the client starts up from that offline refresh token and
//! never sends a password; otherwise it logs in with `ONDEWO_SURVEY_PASSWORD`.
//!
//! ```sh
//! ONDEWO_SURVEY_HOST=survey.example.com ONDEWO_SURVEY_PORT=443 \
//! ONDEWO_KEYCLOAK_URL=https://keycloak.example.com/auth \
//! ONDEWO_KEYCLOAK_REALM=ondewo-ccai-platform \
//! ONDEWO_KEYCLOAK_CLIENT_ID=ondewo-survey-cai-sdk-public \
//! ONDEWO_SURVEY_USERNAME=technical-user@example.com \
//! ONDEWO_SURVEY_OFFLINE_TOKEN=<offline refresh token> \
//! ONDEWO_SURVEY_ID=<survey id> \
//!   cargo run --example keycloak_client
//! ```

use std::env;
use std::error::Error;

use ondewo_survey_client::api::ondewo::survey;
use ondewo_survey_client::api::ondewo::survey::fhir_client::FhirClient;
use ondewo_survey_client::channel::ClientConfig;
use ondewo_survey_client::keycloak::{KeycloakConfig, KeycloakInterceptor};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = KeycloakConfig::new(
        env::var("ONDEWO_KEYCLOAK_URL")?,
        env::var("ONDEWO_KEYCLOAK_REALM")?,
        env::var("ONDEWO_KEYCLOAK_CLIENT_ID")?,
        env::var("ONDEWO_SURVEY_USERNAME").unwrap_or_default(),
        env::var("ONDEWO_SURVEY_PASSWORD").unwrap_or_default(),
    )
    .with_refresh_token(env::var("ONDEWO_SURVEY_OFFLINE_TOKEN").unwrap_or_default());
    // Validates the config, then acquires the first token; the provider keeps it fresh.
    let interceptor = KeycloakInterceptor::new(&config)?;

    let channel = ClientConfig::new(
        env::var("ONDEWO_SURVEY_HOST")?,
        env::var("ONDEWO_SURVEY_PORT")?.parse()?,
    )
    .connect()
    .await?;
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
