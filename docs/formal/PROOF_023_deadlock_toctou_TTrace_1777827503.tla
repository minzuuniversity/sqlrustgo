---- MODULE PROOF_023_deadlock_toctou_TTrace_1777827503 ----
EXTENDS Sequences, TLCExt, PROOF_023_deadlock_toctou, Toolbox, Naturals, TLC

_expression ==
    LET PROOF_023_deadlock_toctou_TEExpression == INSTANCE PROOF_023_deadlock_toctou_TEExpression
    IN PROOF_023_deadlock_toctou_TEExpression!expression
----

_trace ==
    LET PROOF_023_deadlock_toctou_TETrace == INSTANCE PROOF_023_deadlock_toctou_TETrace
    IN PROOF_023_deadlock_toctou_TETrace!trace
----

_inv ==
    ~(
        TLCGet("level") = Len(_TETrace)
        /\
        phase = ([T1 |-> "idle", T2 |-> "idle"])
        /\
        pending = ([T1 |-> {}, T2 |-> {}])
        /\
        waitFor = ([T1 |-> {"T2"}, T2 |-> {"T1"}])
    )
----

_init ==
    /\ pending = _TETrace[1].pending
    /\ phase = _TETrace[1].phase
    /\ waitFor = _TETrace[1].waitFor
----

_next ==
    /\ \E i,j \in DOMAIN _TETrace:
        /\ \/ /\ j = i + 1
              /\ i = TLCGet("level")
        /\ pending  = _TETrace[i].pending
        /\ pending' = _TETrace[j].pending
        /\ phase  = _TETrace[i].phase
        /\ phase' = _TETrace[j].phase
        /\ waitFor  = _TETrace[i].waitFor
        /\ waitFor' = _TETrace[j].waitFor

\* Uncomment the ASSUME below to write the states of the error trace
\* to the given file in Json format. Note that you can pass any tuple
\* to `JsonSerialize`. For example, a sub-sequence of _TETrace.
    \* ASSUME
    \*     LET J == INSTANCE Json
    \*         IN J!JsonSerialize("PROOF_023_deadlock_toctou_TTrace_1777827503.json", _TETrace)

=============================================================================

 Note that you can extract this module `PROOF_023_deadlock_toctou_TEExpression`
  to a dedicated file to reuse `expression` (the module in the 
  dedicated `PROOF_023_deadlock_toctou_TEExpression.tla` file takes precedence 
  over the module `PROOF_023_deadlock_toctou_TEExpression` below).

---- MODULE PROOF_023_deadlock_toctou_TEExpression ----
EXTENDS Sequences, TLCExt, PROOF_023_deadlock_toctou, Toolbox, Naturals, TLC

expression == 
    [
        \* To hide variables of the `PROOF_023_deadlock_toctou` spec from the error trace,
        \* remove the variables below.  The trace will be written in the order
        \* of the fields of this record.
        pending |-> pending
        ,phase |-> phase
        ,waitFor |-> waitFor
        
        \* Put additional constant-, state-, and action-level expressions here:
        \* ,_stateNumber |-> _TEPosition
        \* ,_pendingUnchanged |-> pending = pending'
        
        \* Format the `pending` variable as Json value.
        \* ,_pendingJson |->
        \*     LET J == INSTANCE Json
        \*     IN J!ToJson(pending)
        
        \* Lastly, you may build expressions over arbitrary sets of states by
        \* leveraging the _TETrace operator.  For example, this is how to
        \* count the number of times a spec variable changed up to the current
        \* state in the trace.
        \* ,_pendingModCount |->
        \*     LET F[s \in DOMAIN _TETrace] ==
        \*         IF s = 1 THEN 0
        \*         ELSE IF _TETrace[s].pending # _TETrace[s-1].pending
        \*             THEN 1 + F[s-1] ELSE F[s-1]
        \*     IN F[_TEPosition - 1]
    ]

=============================================================================



Parsing and semantic processing can take forever if the trace below is long.
 In this case, it is advised to uncomment the module below to deserialize the
 trace from a generated binary file.

\*
\*---- MODULE PROOF_023_deadlock_toctou_TETrace ----
\*EXTENDS IOUtils, PROOF_023_deadlock_toctou, TLC
\*
\*trace == IODeserialize("PROOF_023_deadlock_toctou_TTrace_1777827503.bin", TRUE)
\*
\*=============================================================================
\*

---- MODULE PROOF_023_deadlock_toctou_TETrace ----
EXTENDS PROOF_023_deadlock_toctou, TLC

trace == 
    <<
    ([phase |-> [T1 |-> "idle", T2 |-> "idle"],pending |-> [T1 |-> {}, T2 |-> {}],waitFor |-> [T1 |-> {}, T2 |-> {}]]),
    ([phase |-> [T1 |-> "checked", T2 |-> "idle"],pending |-> [T1 |-> {"T2"}, T2 |-> {}],waitFor |-> [T1 |-> {}, T2 |-> {}]]),
    ([phase |-> [T1 |-> "checked", T2 |-> "checked"],pending |-> [T1 |-> {"T2"}, T2 |-> {"T1"}],waitFor |-> [T1 |-> {}, T2 |-> {}]]),
    ([phase |-> [T1 |-> "idle", T2 |-> "checked"],pending |-> [T1 |-> {}, T2 |-> {"T1"}],waitFor |-> [T1 |-> {"T2"}, T2 |-> {}]]),
    ([phase |-> [T1 |-> "idle", T2 |-> "idle"],pending |-> [T1 |-> {}, T2 |-> {}],waitFor |-> [T1 |-> {"T2"}, T2 |-> {"T1"}]])
    >>
----


=============================================================================

---- CONFIG PROOF_023_deadlock_toctou_TTrace_1777827503 ----
CONSTANTS
    Txns = { "T1" , "T2" }

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
\* Generated on Mon May 04 00:58:24 CST 2026