# A static rypsok binary in a distroless image (RFC 013; REQ-DEP-002).
FROM rust:1.88-alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /src
COPY . .
RUN cargo build --release -p rypsok --locked

FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=build /src/target/release/rypsok /rypsok
EXPOSE 8737
USER nonroot
ENTRYPOINT ["/rypsok", "serve", "--transport", "http", "--config", "/etc/rypsok/rypsok.toml"]
