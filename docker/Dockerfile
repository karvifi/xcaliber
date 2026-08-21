# syntax=docker/dockerfile:1
# Xcaliber local Kimi K3 runtime. Model weights are mounted, never copied into the image.

FROM debian:bookworm-slim AS build
ARG ARCH=x86-64-v3
RUN apt-get update && \
    apt-get install -y --no-install-recommends build-essential make libvulkan-dev glslc && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY runtime/colibri/c/ ./c/
RUN make -C c kimi_k3 ARCH="${ARCH}" VK=1

FROM debian:bookworm-slim
ARG XCALIBER_VERSION=1.2.0
LABEL org.opencontainers.image.title="Xcaliber" \
      org.opencontainers.image.description="Local adaptive Kimi K3 runtime" \
      org.opencontainers.image.url="https://github.com/karvifi/xcaliber" \
      org.opencontainers.image.source="https://github.com/karvifi/xcaliber" \
      org.opencontainers.image.version="${XCALIBER_VERSION}" \
      org.opencontainers.image.licenses="AGPL-3.0-only"
RUN apt-get update && \
    apt-get install -y --no-install-recommends python3 libgomp1 libvulkan1 ca-certificates && \
    rm -rf /var/lib/apt/lists/* && \
    useradd --create-home --uid 1000 xcaliber
WORKDIR /app

COPY --from=build /src/c/kimi_k3 ./kimi_k3
COPY --from=build /src/c/shaders/*.spv ./shaders/
COPY runtime/colibri/c/coli \
     runtime/colibri/c/version.py \
     runtime/colibri/c/openai_server.py \
     runtime/colibri/c/v4_dsml.py \
     runtime/colibri/c/family_registry.py \
     runtime/colibri/c/resource_plan.py \
     runtime/colibri/c/doctor.py \
     runtime/colibri/c/autotune.py ./
COPY runtime/colibri/c/tools/ ./tools/
COPY runtime/colibri/LICENSE runtime/colibri/THIRD_PARTY_NOTICES.md ./licenses/colibri/
COPY LICENSE NOTICE ./licenses/xcaliber/

ENV COLI_MODEL=/model \
    COLI_ENGINE=/app/kimi_k3 \
    COLI_VK_SHADERS=/app/shaders/qmatmul.spv \
    PYTHONUNBUFFERED=1 \
    K3_BITS=4 \
    K3_MLA_BITS=8 \
    K3_HEAD_BITS=8 \
    K3_DIRECT=1 \
    K3_PIPE=1 \
    K3_IDOT=1 \
    K3_CHUNK=32 \
    K3_VK=0 \
    K3_VK_GB=0 \
    K3_TOPP=0 \
    COLI_POLICY=quality

VOLUME ["/model"]
EXPOSE 8000
USER xcaliber
ENTRYPOINT ["python3", "/app/coli"]
CMD ["doctor", "--model", "/model"]
