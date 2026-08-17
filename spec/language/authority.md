# Capability, Authority, and Information Flow

Status: **provisional normative boundary**

Runen distinguishes **execution capability** from **security authority**.

Execution capability means that an admitted environment can technically realize an operation. Security authority means that code is permitted to request or perform an operation.

Hardware or environment capability MUST NOT implicitly grant security authority.

Information-flow policy is distinct from ordinary access authority. A security contract may define confidentiality, integrity, release, declassification, endorsement, or related rules without redefining Core ownership or pointer provenance.