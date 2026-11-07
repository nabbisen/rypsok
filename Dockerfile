# A rypsok image made from the release binary (RFC 020, D-12; REQ-DEP-002,
# REQ-DEP-005). The image contains the binary that the release checksums
# name: nothing is compiled here, so there is no second build that could
# differ from the release.
#
# The release workflow builds the static binaries and puts them in
# dist/linux-amd64/rypsok and dist/linux-arm64/rypsok, then runs, once per
# architecture,
#   docker buildx build --platform linux/<arch> .
# To make an image yourself, build the binary for the musl target and copy it
# to dist/linux-<arch>/rypsok first (docs/src/maintainers/releasing.md).
#
# gcr.io/distroless/static-debian12:nonroot, read on 2026-11-03 (the digest is
# the multi-platform index; a new digest is a deliberate change).
FROM gcr.io/distroless/static-debian12@sha256:afa5c872c891853ca7fcf1f12c3edb23f7eeef36189728842dd51042ff57f7ab
ARG TARGETARCH
COPY dist/linux-${TARGETARCH}/rypsok /rypsok
EXPOSE 8737
USER nonroot
ENTRYPOINT ["/rypsok", "serve", "--transport", "http", "--config", "/etc/rypsok/rypsok.toml"]
