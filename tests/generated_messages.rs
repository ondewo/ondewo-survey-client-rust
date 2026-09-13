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

//! Wire-level tests for the GENERATED prost messages under `src/api`.
//!
//! These are the cases that catch a broken generator: a dropped field, a shifted tag number, an
//! enum whose discriminants moved, a oneof that lost a variant. They are pure encode/decode - no
//! runtime, no socket. The gRPC plumbing is covered by `tests/generated_grpc.rs`.
//!
//! NOTE on explicit presence: the ONDEWO NLU rust client additionally asserts that a proto3
//! `optional` scalar stays distinguishable from its zero value. `ondewo-survey-api` declares no
//! such field anywhere - neither in `ondewo.survey` nor in the vendored `google.api` - so there is
//! nothing here to assert it against and the case is deliberately absent rather than faked against
//! a message-typed field (those are `Option<T>` in prost regardless of presence and prove nothing).

use ondewo_survey_client::api::google;
use ondewo_survey_client::api::ondewo::survey;
use prost::Message;

/// A fully populated [`survey::Survey`] - scalar, repeated message, nested message and both
/// enum shapes (a singular enum field and a repeated one) at once.
fn sample_survey() -> survey::Survey {
    survey::Survey {
        survey_id: "survey-42".to_string(),
        display_name: "Customer satisfaction".to_string(),
        language_code: "de".to_string(),
        questions: vec![
            survey::Question {
                question: Some(survey::question::Question::OpenQuestion(
                    survey::OpenQuestion {
                        question_text: "How did we do?".to_string(),
                    },
                )),
            },
            survey::Question {
                question: Some(survey::question::Question::ScaleQuestion(
                    survey::ScaleQuestion {
                        question_text: "Rate us".to_string(),
                        min_value: Some(survey::scale_question::ScaleValue {
                            value: 1,
                            label: "poor".to_string(),
                        }),
                        max_value: Some(survey::scale_question::ScaleValue {
                            value: 5,
                            label: "excellent".to_string(),
                        }),
                    },
                )),
            },
        ],
        survey_info: Some(survey::SurveyInfo {
            legal_entity: "ONDEWO GmbH".to_string(),
            email_address: "office@ondewo.com".to_string(),
            expected_duration: "3 minutes".to_string(),
            anonymous: true,
            ..Default::default()
        }),
        exclude_subflows: vec![
            survey::SubFlow::PhoneHours as i32,
            survey::SubFlow::Purpose as i32,
        ],
        status: survey::survey::AgentStatus::Updated as i32,
    }
}

#[test]
fn survey_survives_a_serialize_parse_round_trip() {
    let original = sample_survey();

    let bytes = original.encode_to_vec();
    assert!(
        !bytes.is_empty(),
        "a populated Survey must not encode to zero bytes"
    );
    assert_eq!(
        bytes.len(),
        original.encoded_len(),
        "encoded_len must agree with the bytes actually written"
    );

    let parsed =
        survey::Survey::decode(bytes.as_slice()).expect("re-parsing our own bytes must work");
    assert_eq!(parsed, original);

    // Spot-check the individual fields too: a PartialEq on two identically broken values would
    // still pass above.
    assert_eq!(parsed.survey_id, "survey-42");
    assert_eq!(parsed.questions.len(), 2);
    assert_eq!(
        parsed.questions[0].question,
        Some(survey::question::Question::OpenQuestion(
            survey::OpenQuestion {
                question_text: "How did we do?".to_string(),
            }
        ))
    );
    assert!(parsed.survey_info.as_ref().unwrap().anonymous);
    assert_eq!(
        parsed.exclude_subflows,
        vec![
            survey::SubFlow::PhoneHours as i32,
            survey::SubFlow::Purpose as i32
        ]
    );
}

#[test]
fn a_default_survey_round_trips_to_zero_bytes() {
    let empty = survey::Survey::default();

    assert_eq!(empty.survey_id, "");
    assert_eq!(empty.status, 0);
    assert!(empty.questions.is_empty());
    assert_eq!(empty.survey_info, None);

    let bytes = empty.encode_to_vec();
    assert!(
        bytes.is_empty(),
        "proto3 must not put unset fields on the wire, got {bytes:?}"
    );
    assert_eq!(survey::Survey::decode(bytes.as_slice()).unwrap(), empty);
}

/// Decoding tolerates fields it does not know: an unknown tag is skipped, not an error.
#[test]
fn decoding_skips_an_unknown_field() {
    let mut bytes = survey::GetSurveyRequest {
        survey_id: "survey-42".to_string(),
    }
    .encode_to_vec();
    // tag 999, wire type 0 (varint), value 1
    bytes.extend_from_slice(&[0xB8, 0x3E, 0x01]);

    let parsed = survey::GetSurveyRequest::decode(bytes.as_slice())
        .expect("an unknown field must be skipped, not rejected");
    assert_eq!(parsed.survey_id, "survey-42");
}

#[test]
fn decoding_rejects_a_truncated_message() {
    let bytes = sample_survey().encode_to_vec();
    let truncated = &bytes[..bytes.len() - 1];

    assert!(
        survey::Survey::decode(truncated).is_err(),
        "a truncated message must not decode silently"
    );
}

/// The zero value of an enum is the one a default-constructed message carries, so it must be the
/// variant the proto declares as `= 0`.
#[test]
fn the_enum_zero_value_is_the_variant_the_proto_declares_as_zero() {
    assert_eq!(survey::SubFlow::SubflowUnspecified as i32, 0);
    assert_eq!(
        survey::SubFlow::try_from(0),
        Ok(survey::SubFlow::SubflowUnspecified)
    );
    assert_eq!(
        survey::SubFlow::SubflowUnspecified.as_str_name(),
        "SUBFLOW_UNSPECIFIED"
    );
    assert_eq!(
        survey::SubFlow::from_str_name("SUBFLOW_UNSPECIFIED"),
        Some(survey::SubFlow::SubflowUnspecified)
    );
    assert_eq!(survey::SubFlow::from_str_name("NOT_A_VARIANT"), None);
    assert!(
        survey::SubFlow::try_from(9_999).is_err(),
        "an out-of-range discriminant must not map to a variant"
    );

    // `Survey.status` is an `AgentStatus`, whose zero variant is TO_BE_INITIALIZED rather than an
    // UNSPECIFIED - a default message has to carry exactly that.
    assert_eq!(survey::survey::AgentStatus::ToBeInitialized as i32, 0);
    assert_eq!(
        survey::Survey::default().status,
        survey::survey::AgentStatus::ToBeInitialized as i32,
        "a default message must carry the enum's zero value"
    );
    assert_eq!(
        survey::survey::AgentStatus::ToBeInitialized.as_str_name(),
        "TO_BE_INITIALIZED"
    );
}

/// A non-zero enum value has to travel as its discriminant, not as the zero value.
#[test]
fn a_non_zero_enum_value_round_trips() {
    let original = survey::Survey {
        survey_id: "survey-42".to_string(),
        status: survey::survey::AgentStatus::Outdated as i32,
        exclude_subflows: vec![survey::SubFlow::Bot as i32],
        ..Default::default()
    };

    let parsed = survey::Survey::decode(original.encode_to_vec().as_slice()).unwrap();
    assert_eq!(parsed, original);
    assert_eq!(
        survey::survey::AgentStatus::try_from(parsed.status),
        Ok(survey::survey::AgentStatus::Outdated)
    );
    assert_eq!(
        survey::SubFlow::try_from(parsed.exclude_subflows[0]),
        Ok(survey::SubFlow::Bot)
    );
}

/// Repeated and nested message fields have to nest, not flatten.
#[test]
fn a_nested_and_repeated_message_round_trips() {
    let response = survey::ListSurveysResponse {
        surveys: vec![
            sample_survey(),
            survey::Survey {
                survey_id: "second".to_string(),
                ..Default::default()
            },
        ],
        next_page_token: "current_index-1--page_size-20".to_string(),
    };

    let parsed = survey::ListSurveysResponse::decode(response.encode_to_vec().as_slice()).unwrap();
    assert_eq!(parsed, response);
    assert_eq!(parsed.surveys.len(), 2);
    assert_eq!(parsed.surveys[1].survey_id, "second");
}

/// A oneof carries exactly one variant, and an unset oneof is not the same as its first variant
/// holding a default value.
#[test]
fn a_oneof_keeps_the_variant_it_was_given() {
    let by_session = survey::GetSurveyAnswersRequest {
        survey_id: "survey-42".to_string(),
        identifier: Some(survey::get_survey_answers_request::Identifier::SessionId(
            "session-1".to_string(),
        )),
    };
    let by_user = survey::GetSurveyAnswersRequest {
        survey_id: "survey-42".to_string(),
        identifier: Some(survey::get_survey_answers_request::Identifier::UserId(
            "session-1".to_string(),
        )),
    };

    // Same payload, different variant - the tag number is what distinguishes them on the wire.
    assert_ne!(by_session.encode_to_vec(), by_user.encode_to_vec());
    assert_eq!(
        survey::GetSurveyAnswersRequest::decode(by_session.encode_to_vec().as_slice()).unwrap(),
        by_session
    );
    assert_eq!(
        survey::GetSurveyAnswersRequest::decode(by_user.encode_to_vec().as_slice()).unwrap(),
        by_user
    );

    let unset = survey::GetSurveyAnswersRequest {
        survey_id: "survey-42".to_string(),
        identifier: None,
    };
    assert_ne!(unset.encode_to_vec(), by_session.encode_to_vec());
}

/// The generator emits one module per proto PACKAGE. `ondewo-survey-api` vendors exactly one
/// `google.*` package - `google.api`, pulled in by the `google.api.http` annotations on the
/// services - and its messages have to be reachable and usable too.
#[test]
fn messages_of_every_generated_package_are_reachable() {
    let pattern = google::api::CustomHttpPattern {
        kind: "GET".to_string(),
        path: "/v2/surveys".to_string(),
    };
    let parsed =
        google::api::CustomHttpPattern::decode(pattern.encode_to_vec().as_slice()).unwrap();
    assert_eq!(parsed, pattern);
    assert_eq!(parsed.kind, "GET");

    let rule = google::api::HttpRule {
        selector: "ondewo.survey.Surveys.ListSurveys".to_string(),
        body: "*".to_string(),
        pattern: Some(google::api::http_rule::Pattern::Custom(pattern)),
        ..Default::default()
    };
    assert_eq!(
        google::api::HttpRule::decode(rule.encode_to_vec().as_slice()).unwrap(),
        rule
    );
}
