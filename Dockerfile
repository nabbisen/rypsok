# A static rypsok binary in a distroless image (RFC 013; REQ-DEP-002).
# rust:1.88-alpine, read from the registry on 2026-11-03 (the digest is the
# multi-platform index; a new digest is a deliberate change).
FROM rust:1.88-alpine@sha256:9dfaae478ecd298b6b5a039e1f2cc4fc040fc818a2de9aa78fa714dea036574d AS build
RUN apk add --no-cache musl-dev
WORKDIR /src
COPY . .
RUN cargo build --release -p rypsok --locked

# gcr.io/distroless/static-debian12:nonroot, read on 2026-11-03.
FROM gcr.io/distroless/static-debian12@sha256:afa5c872c891853ca7fcf1f12c3edb23f7eeef36189728842dd51042ff57f7ab
COPY --from=build /src/target/release/rypsok /rypsok
EXPOSE 8737
USER nonroot
ENTRYPOINT ["/rypsok", "serve", "--transport", "http", "--config", "/etc/rypsok/rypsok.toml"]
