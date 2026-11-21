# Release History

*****************

## Release ONDEWO SURVEY Rust Client 2.1.1

### New Features

* [[OND236-120]](https://ondewo.atlassian.net/browse/OND236-120) Keycloak authentication in the new module `keycloak`: `KeycloakConfig` (password or a handed-off offline `refresh_token`, appended last, redacted `Debug`, all-or-nothing validation), `KeycloakTokenProvider` with a background refresh thread and a bounded stop, a weak SHA-256-keyed shared registry, and `KeycloakInterceptor`, which attaches `authorization: Bearer <token>` to every call. `examples/keycloak_client.rs` shows the wiring.
* [[OND236-120]](https://ondewo.atlassian.net/browse/OND236-120) Offline-token start-up: with `refresh_token` set the provider starts with one `grant_type=refresh_token` exchange and never sends a password grant, not even as a fallback. The default transport is one blocking HTTP/1.1 POST over std sockets and rustls (built without its default features).
* [[OND236-120]](https://ondewo.atlassian.net/browse/OND236-120) Tests: the cross-SDK Keycloak test catalogue (T01-T62) plus HTTP and TLS transport cases; the hand-written code stays at 100 % line coverage.

### Bug Fixes

* [[OND236-120]](https://ondewo.atlassian.net/browse/OND236-120) Keycloak token refresh: a failed refresh no longer ends the refresh. Every failure kind (a 5xx from a proxy, a DNS or connection failure, a restarting Keycloak, a dead offline session) is retried with full-jitter exponential backoff, 5 s doubling up to 300 s and measured from the moment the failure was recorded; the background refresh thread and the lazy path share that backoff. A success resets the backoff.
* [[OND236-120]](https://ondewo.atlassian.net/browse/OND236-120) A token that is still valid keeps being served when its refresh fails inside the 30 s expiry leeway; only an EXPIRED token raises, and it raises a typed `KeycloakAuthenticationError` carrying its source whatever failed underneath (transport error, non-2xx status, a body that is not JSON or not a JSON object, a missing `access_token`, a non-numeric `expires_in`), with the original error kept as its cause. A malformed response changes no provider state. Concurrent callers share one in-flight refresh, so an outage costs one token request, not one per call.
* [[OND236-120]](https://ondewo.atlassian.net/browse/OND236-120) A token response without an `access_token` is reported by its field names only, never by its values (a token response is credential material), and a quoted Keycloak error body is cut at 500 characters.

### Improvements

* [[OND236-120]](https://ondewo.atlassian.net/browse/OND236-120) The same Keycloak behaviour as every ONDEWO SDK (the python SDKs are the reference, ondewo-survey-client-python 2.1.1 ships the same fixes): the token request is bounded by a 10 s timeout, durations use a monotonic clock, the `token_expiration_in_s` window is armed on both start-up paths, each failed refresh logs one WARNING with the attempt number, the retry delay and how long the current token stays valid, and the first success afterwards one INFO. No credential appears in any log line, error or rendering of the configuration.
* Tracking API Version [2.1.0](https://github.com/ondewo/ondewo-survey-api/releases/tag/2.1.0) ( [Documentation](https://ondewo.github.io/ondewo-survey-api/) ), generated with ondewo-proto-compiler 5.17.0: the generated code is unchanged from 2.1.0

*****************

## Release ONDEWO SURVEY Rust Client 2.1.0

### Improvements

* Tracking API Version [2.1.0](https://github.com/ondewo/ondewo-survey-api/releases/tag/2.1.0) ( [Documentation](https://ondewo.github.io/ondewo-survey-api/) )
* Generated with ondewo-proto-compiler 5.17.0 (protoc 35.1). New runtime floors: `prost` / `prost-types` 0.14.6 and `tonic` / `tonic-prost` 0.14.9.

*****************

## Release ONDEWO SURVEY Rust Client 2.0.1

### New Features

* TLS, mutual TLS and plaintext channels: the new hand-written module `channel` builds the tonic
  `Endpoint` / `Channel` from a `ClientConfig` (`endpoint()`, `connect()`, `connect_lazy()`).
  `grpc_cert`, `grpc_client_cert` and `grpc_client_key` take PEM **content**, not file paths.
  Without `grpc_cert` the platform trust store is used (tonic `tls-native-roots`).
* Refused with a `ChannelError` before tonic sees the config: half a client identity
  (`IncompleteClientIdentity`), a client identity on a plaintext channel
  (`ClientIdentityWithoutTls`) and a certificate field that holds no PEM certificate
  (`NotAPemCertificate`).
* A plaintext channel logs a `tracing` warning naming `host:port`. `Debug` redacts the client key
  and renders no PEM, and error messages name only the field and `host:port`. A bare IPv6 host is
  bracketed, and `tls_domain_name` overrides the name the server certificate is checked against.
* README section "TLS, mutual TLS and certificates": the modes, a test PKI built with openssl, TLS
  security notes and the channel defaults. Of the python SDK channel options, the message size
  limit is applied per client (`MAX_MESSAGE_LENGTH`); keepalive pings are not set (TCP keepalive
  detects a dropped idle connection instead), and tonic has no reconnect backoff or retry policy.

### Tests

* `tests/tls_channel.rs` runs real handshakes against an in-process tonic server with an
  openssl-generated test PKI: plain TLS, mutual TLS, missing and foreign client identities, wrong
  CA, platform roots, CRLF PEMs, SAN override and IPv6 loopback.
* `tests/release_notes.rs` pins the RELEASE.md slice the GitHub release body is cut from.

### Build

* `ondewo-proto-compiler` is pinned to 5.15.4 (5.15.2 before this release), which pre-warms tonic
  `tls-native-roots` for the offline stub generation that `src/channel.rs` needs.
  `make generate_ondewo_protos` now also stages `README.md`, which the image's `cargo package`
  requires. The regenerated stubs under `src/api` are byte-identical.
* Tracking API Version [2.0.0](https://github.com/ondewo/ondewo-survey-api/releases/tag/2.0.0) (unchanged)

*****************

## Release ONDEWO SURVEY Rust Client 2.0.0

### New Features

* Initial release of the ONDEWO SURVEY (Survey) gRPC client for rust. The crate
  ships the complete client surface of the [ondewo-survey-api](https://github.com/ondewo/ondewo-survey-api)
  protocol buffer definitions: [prost](https://crates.io/crates/prost) message types and
  [tonic](https://crates.io/crates/tonic) service clients for every RPC, generated by the
  `ondewo-rust-proto-compiler` image of
  [ondewo-proto-compiler](https://github.com/ondewo/ondewo-proto-compiler) 5.15.1.
* The generated module tree is confined to `src/api` and is re-exported from the crate root, so a
  message declared in `package ondewo.survey;` is reachable as
  `ondewo_survey_client::ondewo::survey::<Message>` and each service as
  `ondewo::survey::<service>_client::<Service>Client`. Hand-written modules live beside it
  under `src/` and are declared in the hand-written crate barrel `src/lib.rs`, which the
  generator leaves untouched.
* The generated stubs under `src/api` are COMMITTED, so the crate builds straight from a checkout -
  no docker, no compiler image and no submodule needed. CI compiles and tests exactly those files.
* A test suite under `tests/` exercises the generated stubs rather than only the hand-written code:
  prost messages are serialized and re-parsed field by field, oneofs and enum discriminants are
  pinned, and the generated `FhirServer` - all three unary RPCs of `ondewo.survey.FHIR` - is served
  over a loopback socket and driven by the generated `FhirClient`, so every declared RPC really is
  encoded, routed by its `/ondewo.survey.FHIR/<Method>` path, answered and decoded again.
  `ondewo-survey-api` declares no proto3 `optional` scalar anywhere, so there is no
  explicit-presence case to assert; a `google.protobuf.Struct` round trip takes its place.
* A `BearerTokenInterceptor` (`src/auth.rs`) attaches the Keycloak `authorization` and `cai-token`
  metadata to every request; its `Debug` output redacts both credentials. `make coverage` gates the
  hand-written sources at 100% line coverage and the same gate runs in CI.
* `make build` runs the whole pipeline - submodule checkout, compiler image build, stub
  generation and `cargo build` - and `make check_build` asserts that a generated stub exists for
  every proto package before a release is cut.

*****************
