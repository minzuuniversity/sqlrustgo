---- MODULE PROOF_016_023_mvcc_atomic_TTrace_1777795872 ----
EXTENDS PROOF_016_023_mvcc_atomic, Sequences, TLCExt, Toolbox, Naturals, TLC

_expression ==
    LET PROOF_016_023_mvcc_atomic_TEExpression == INSTANCE PROOF_016_023_mvcc_atomic_TEExpression
    IN PROOF_016_023_mvcc_atomic_TEExpression!expression
----

_trace ==
    LET PROOF_016_023_mvcc_atomic_TETrace == INSTANCE PROOF_016_023_mvcc_atomic_TETrace
    IN PROOF_016_023_mvcc_atomic_TETrace!trace
----

_inv ==
    ~(
        TLCGet("level") = Len(_TETrace)
        /\
        committed = ({"T1", "T2"})
        /\
        writeSet = ([T1 |-> {"K1"}, T2 |-> {"K1"}])
        /\
        readSet = ([T1 |-> {}, T2 |-> {}])
        /\
        waitFor = ([T1 |-> {}, T2 |-> {}])
        /\
        ts = ([T1 |-> "T1", T2 |-> "T2"])
    )
----

_init ==
    /\ writeSet = _TETrace[1].writeSet
    /\ ts = _TETrace[1].ts
    /\ committed = _TETrace[1].committed
    /\ waitFor = _TETrace[1].waitFor
    /\ readSet = _TETrace[1].readSet
----

_next ==
    /\ \E i,j \in DOMAIN _TETrace:
        /\ \/ /\ j = i + 1
              /\ i = TLCGet("level")
        /\ writeSet  = _TETrace[i].writeSet
        /\ writeSet' = _TETrace[j].writeSet
        /\ ts  = _TETrace[i].ts
        /\ ts' = _TETrace[j].ts
        /\ committed  = _TETrace[i].committed
        /\ committed' = _TETrace[j].committed
        /\ waitFor  = _TETrace[i].waitFor
        /\ waitFor' = _TETrace[j].waitFor
        /\ readSet  = _TETrace[i].readSet
        /\ readSet' = _TETrace[j].readSet

\* Uncomment the ASSUME below to write the states of the error trace
\* to the given file in Json format. Note that you can pass any tuple
\* to `JsonSerialize`. For example, a sub-sequence of _TETrace.
    \* ASSUME
    \*     LET J == INSTANCE Json
    \*         IN J!JsonSerialize("PROOF_016_023_mvcc_atomic_TTrace_1777795872.json", _TETrace)

=============================================================================

 Note that you can extract this module `PROOF_016_023_mvcc_atomic_TEExpression`
  to a dedicated file to reuse `expression` (the module in the 
  dedicated `PROOF_016_023_mvcc_atomic_TEExpression.tla` file takes precedence 
  over the module `PROOF_016_023_mvcc_atomic_TEExpression` below).

---- MODULE PROOF_016_023_mvcc_atomic_TEExpression ----
EXTENDS PROOF_016_023_mvcc_atomic, Sequences, TLCExt, Toolbox, Naturals, TLC

expression == 
    [
        \* To hide variables of the `PROOF_016_023_mvcc_atomic` spec from the error trace,
        \* remove the variables below.  The trace will be written in the order
        \* of the fields of this record.
        writeSet |-> writeSet
        ,ts |-> ts
        ,committed |-> committed
        ,waitFor |-> waitFor
        ,readSet |-> readSet
        
        \* Put additional constant-, state-, and action-level expressions here:
        \* ,_stateNumber |-> _TEPosition
        \* ,_writeSetUnchanged |-> writeSet = writeSet'
        
        \* Format the `writeSet` variable as Json value.
        \* ,_writeSetJson |->
        \*     LET J == INSTANCE Json
        \*     IN J!ToJson(writeSet)
        
        \* Lastly, you may build expressions over arbitrary sets of states by
        \* leveraging the _TETrace operator.  For example, this is how to
        \* count the number of times a spec variable changed up to the current
        \* state in the trace.
        \* ,_writeSetModCount |->
        \*     LET F[s \in DOMAIN _TETrace] ==
        \*         IF s = 1 THEN 0
        \*         ELSE IF _TETrace[s].writeSet # _TETrace[s-1].writeSet
        \*             THEN 1 + F[s-1] ELSE F[s-1]
        \*     IN F[_TEPosition - 1]
    ]

=============================================================================



Parsing and semantic processing can take forever if the trace below is long.
 In this case, it is advised to uncomment the module below to deserialize the
 trace from a generated binary file.

\*
\*---- MODULE PROOF_016_023_mvcc_atomic_TETrace ----
\*EXTENDS PROOF_016_023_mvcc_atomic, IOUtils, TLC
\*
\*trace == IODeserialize("PROOF_016_023_mvcc_atomic_TTrace_1777795872.bin", TRUE)
\*
\*=============================================================================
\*

---- MODULE PROOF_016_023_mvcc_atomic_TETrace ----
EXTENDS PROOF_016_023_mvcc_atomic, TLC

trace == 
    <<
    ([committed |-> {},writeSet |-> [T1 |-> {}, T2 |-> {}],readSet |-> [T1 |-> {}, T2 |-> {}],waitFor |-> [T1 |-> {}, T2 |-> {}],ts |-> [T1 |-> "T1", T2 |-> "T2"]]),
    ([committed |-> {"T1"},writeSet |-> [T1 |-> {}, T2 |-> {}],readSet |-> [T1 |-> {}, T2 |-> {}],waitFor |-> [T1 |-> {}, T2 |-> {}],ts |-> [T1 |-> "T1", T2 |-> "T2"]]),
    ([committed |-> {"T1"},writeSet |-> [T1 |-> {"K1"}, T2 |-> {}],readSet |-> [T1 |-> {}, T2 |-> {}],waitFor |-> [T1 |-> {}, T2 |-> {}],ts |-> [T1 |-> "T1", T2 |-> "T2"]]),
    ([committed |-> {"T1", "T2"},writeSet |-> [T1 |-> {"K1"}, T2 |-> {}],readSet |-> [T1 |-> {}, T2 |-> {}],waitFor |-> [T1 |-> {}, T2 |-> {}],ts |-> [T1 |-> "T1", T2 |-> "T2"]]),
    ([committed |-> {"T1", "T2"},writeSet |-> [T1 |-> {"K1"}, T2 |-> {"K1"}],readSet |-> [T1 |-> {}, T2 |-> {}],waitFor |-> [T1 |-> {}, T2 |-> {}],ts |-> [T1 |-> "T1", T2 |-> "T2"]])
    >>
----


=============================================================================

---- CONFIG PROOF_016_023_mvcc_atomic_TTrace_1777795872 ----
CONSTANTS
    Txns = { "T1" , "T2" }
    Keys = { "K1" }

INVARIANT
    _inv

CHECK_DEADLOCK
    \* CHECK_DEADLOCK off because of PROPERTY or INVARIANT above.
    FALSE

INIT
    _init

NEXT
    _next

CONSTANT
    _TETrace <- _trace

ALIAS
    _expression
=============================================================================
\* Generated on Sun May 03 16:11:13 CST 2026