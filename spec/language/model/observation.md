# Model Observation

Status: **provisional normative; incomplete**

A Model evaluation spanning state domains is evaluated relative to an explicit immutable `ObservationSet` identifying the admitted observations for that evaluation or reaction wave.

An `ObservationSet` is immutable for that wave.

An `ObservationSet` does not imply one globally synchronized distributed snapshot unless a stronger contract explicitly guarantees one.

`observe` requests logical observation semantics; it does not mandate one incremental realization.

The compatibility and admission rules for composing observations from multiple state domains are not defined by this revision.