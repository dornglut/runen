# Research Basis

Status: **non-normative research record**

External work is used to pressure-test Runen design; it is not Runen specification authority.

Current high-signal references and the questions they pressure-test include:

- CompCert — observable behavior and refinement;
- Iris / RustBelt — unsafe abstraction and resource soundness;
- CHERI — separation of address, provenance, bounds, permission, and pointer capability;
- Koka — effect inference;
- Deterministic Parallel Java — noninterference and deterministic parallelism;
- Regent / Legion — logical resources, privileges, and physical instances;
- WGSL / SPIR-V — heterogeneous numeric, memory, and scope constraints;
- Lustre / Vélus — logical time and verified lowering;
- self-adjusting computation — from-scratch versus incremental equivalence;
- Jif — authority versus information flow;
- TLA+ — state/progress/fairness protocol reasoning;
- Alive2 — translation-validation precedent;
- WebAssembly Core Specification — validation/execution/embedding specification structure.

Research notes should record what question a source informs rather than copying another system's mechanism into Runen by analogy.