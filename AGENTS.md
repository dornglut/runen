# Agent Instructions

Automated contributors must follow [CONTRIBUTING.md](CONTRIBUTING.md) and the repository [Documentation Architecture](docs/documentation-architecture.md).

Before editing, inspect the canonical owner of the concern and its direct normative dependencies.

For iterative continuation, follow Dornglut's [Authority and work](https://github.com/dornglut/engineering/blob/main/governance/authority-and-work.md) rules. Re-establish current repository state before continuing: resume specifically selected accepted work unless accepted changes materially affect its ownership, dependencies, assumptions, acceptance boundary, or mergeability; when selecting new work, derive it from accepted repository authority rather than prior conversation, handoff, or agent planning state. An open specification item is not by itself the next roadmap slice; unresolved design uncertainty belongs in investigation rather than invented implementation, and autonomous continuation stops when no justified next slice is established.

Do not complete an open semantic rule from host-language behavior, compiler convenience, analogy to another language, or test expectations. Record the gap or work under the owning issue instead.

Do not create compatibility aliases, duplicate authorities, generated planning state, or speculative abstractions unless the owning issue explicitly requires them.

Before proposing acceptance, run the canonical repository validation and review the exact changed head.