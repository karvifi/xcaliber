# Research and performance boundary

Research date: 2026-08-21

## Model facts

Moonshot's official repository/config describe Kimi K3 as a 2.8T-parameter MoE with
about 104B active parameters, 93 layers, 896 routed experts with 16 selected, native
MXFP4/MXFP8 weights, and up to a 1M-token context. Sparse activation cuts arithmetic;
it does not remove the 1.56 TB checkpoint requirement.

Primary references:

- [MoonshotAI/Kimi-K3](https://github.com/MoonshotAI/Kimi-K3)
- [Official Kimi K3 config](https://huggingface.co/moonshotai/Kimi-K3/blob/main/config.json)
- [vLLM K3 implementation](https://docs.vllm.ai/en/latest/api/vllm/models/kimi_k3/)

## Measured local boundary

The bundled exact Rust reference's upstream full-model record reports 72.4722 seconds
per token at an 8 GiB cap on a 16-vCPU Arm host, with 82.8% of time attributed to I/O.
The record is preserved at `runtime/rust/docs/data/full-model.csv`. The exact streamed
path moves roughly 108.81 GB of dense trunk plus up to 25.83 GB of selected expert data
per token. Two tokens per second would therefore need roughly 269 GB/s of effective
stream bandwidth before compute. An ordinary SSD is not close.

The `xcaliber plan --measure-io` command measures a 512 MB sequential sample on the selected
drive and reports an upper bound. It labels the sample as cache- and temperature-
sensitive; it is not a benchmark certification.

## Techniques considered

| Technique | Why it can help | Xcaliber status | Semantic effect |
|---|---|---|---|
| Expert RAM/VRAM LRU | Avoid repeated SSD reads for hot experts | Colibri engine | Exact expert bytes; dense path still quantized |
| Async read/prefetch | Overlap I/O and matrix work | Colibri engine | Lossless |
| O_DIRECT with fallback | Avoid double caching and cache pollution | Colibri engine | Lossless |
| Vulkan hot tier | Keep selected experts in small VRAM | Colibri engine, opt-in | Lossless for stored expert representation |
| Verified n-gram drafts | Batch repeated-token verification | Native CLI default | Exact output |
| Quantized dense residency | Removes 108.81 GB/token trunk streaming | Docker adaptive mode | Changes numerical precision |
| Expert top-p pruning | Reads/computes fewer experts | Available but off by default | Changes routing result |
| Smaller/distilled model | Fits 32 GB/6 GB and can be interactive | Not mislabeled as K3 | Different model |
| Several ordinary PCs | Aggregate RAM, storage, and bandwidth | K3 protocol not implemented | Needs new parity work |

Relevant primary research/code:

- [MoE-Infinity](https://arxiv.org/abs/2401.14361) and
  [FlashMoE](https://arxiv.org/abs/2601.17063): expert offload, cache, and prefetch.
- [FlexGen](https://arxiv.org/abs/2303.06865) and
  [PowerInfer-2](https://arxiv.org/abs/2406.06282): multi-tier inference under memory limits.
- [SpecOffload](https://arxiv.org/abs/2505.10259): speculative decoding with offloaded models.
- [KTransformers](https://github.com/kvcache-ai/ktransformers): heterogeneous CPU/GPU inference.
- [Petals](https://github.com/bigscience-workshop/petals) and
  [exo](https://github.com/exo-explore/exo): distributed inference across multiple systems.

## Decisions

1. Keep the exact streamed path intact for official numerical semantics.
2. Expose adaptive quantization as a separately named mode, never as exact K3.
3. Target 2 token/s only as a planner threshold, not a promise.
4. Do not add a K3 cluster checkbox until worker sharding, failure recovery, routing
   determinism, and full-model parity tests exist.
5. On a 32 GB/6 GB laptop with insufficient model storage, recommend a clearly labeled
   smaller local model or additional hardware; no setting can manufacture the missing
   capacity and bandwidth.
