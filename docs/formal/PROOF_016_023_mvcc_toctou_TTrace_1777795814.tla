---- MODULE PROOF_016_023_mvcc_toctou_TTrace_1777795814 ----
EXTENDS Sequences, TLCExt, Toolbox, Naturals, TLC, PROOF_016_023_mvcc_toctou

_expression ==
    LET PROOF_016_023_mvcc_toctou_TEExpression == INSTANCE PROOF_016_023_mvcc_toctou_TEExpression
    IN PROOF_016_023_mvcc_toctou_TEExpression!expression
----

_trace ==
    LET PROOF_016_023_mvcc_toctou_TETrace == INSTANCE PROOF_016_023_mvcc_toctou_TETrace
    IN PROOF_016_023_mvcc_toctou_TETrace!trace
----

_inv ==
    ~(
        TLCGet("level") = Len(_TETrace)
        /\
        phase = ([T1 |-> "idle", T2 |-> "idle"])
        /\
        committed = ({})
        /\
        writeSet = ([T1 |-> {}, T2 |-> {}])
        /\
        pending = ([T1 |-> {}, T2 |-> {}])
        /\
        readSet = ([T1 |-> {}, T2 |-> {}])
        /\
        waitFor = ([T1 |-> {"T2"}, T2 |-> {"T1"}])
        /\
        ts = ([T1 |-> "T1", T2 |-> "T2"])
    )
----

_init ==
    /\ pending = _TETrace[1].pending
    /\ writeSet = _TETrace[1].writeSet
    /\ ts = _TETrace[1].ts
    /\ committed = _TETrace[1].committed
    /\ phase = _TETrace[1].phase
    /\ waitFor = _TETrace[1].waitFor
    /\ readSet = _TETrace[1].readSet
----

_next ==
    /\ \E i,j \in DOMAIN _TETrace:
        /\ \/ /\ j = i + 1
              /\ i = TLCGet("level")
        /\ pending  = _TETrace[i].pending
        /\ pending' = _TETrace[j].pending
        /\ writeSet  = _TETrace[i].writeSet
        /\ writeSet' = _TETrace[j].writeSet
        /\ ts  = _TETrace[i].ts
        /\ ts' = _TETrace[j].ts
        /\ committed  = _TETrace[i].committed
        /\ committed' = _TETrace[j].committed
        /\ phase  = _TETrace[i].phase
        /\ phase' = _TETrace[j].phase
        /\ waitFor  = _TETrace[i].waitFor
        /\ waitFor' = _TETrace[j].waitFor
        /\ readSet  = _TETrace[i].readSet
        /\ readSet' = _TETrace[j].readSet

\* Uncomment the ASSUME below to write the states of the error trace
\* to the given file in Json format. Note that you can pass any tuple
\* to `JsonSerialize`. For example, a sub-sequence of _TETrace.
    \* ASSUME
    \*     LET J == INSTANCE Json
    \*         IN J!JsonSerialize("PROOF_016_023_mvcc_toctou_TTrace_1777795814.json", _TETrace)

=============================================================================

 Note that you can extract this module `PROOF_016_023_mvcc_toctou_TEExpression`
  to a dedicated file to reuse `expression` (the module in the 
  dedicated `PROOF_016_023_mvcc_toctou_TEExpression.tla` file takes precedence 
  over the module `PROOF_016_023_mvcc_toctou_TEExpression` below).

---- MODULE PROOF_016_023_mvcc_toctou_TEExpression ----
EXTENDS Sequences, TLCExt, Toolbox, Naturals, TLC, PROOF_016_023_mvcc_toctou

expression == 
    [
        \* To hide variables of the `PROOF_016_023_mvcc_toctou` spec from the error trace,
        \* remove the variables below.  The trace will be written in the order
        \* of the fields of this record.
        pending |-> pending
        ,writeSet |-> writeSet
        ,ts |-> ts
        ,committed |-> committed
        ,phase |-> phase
        ,waitFor |-> waitFor
        ,readSet |-> readSet
        
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
\*---- MODULE PROOF_016_023_mvcc_toctou_TETrace ----
\*EXTENDS IOUtils, TLC, PROOF_016_023_mvcc_toctou
\*
\*trace == IODeserialize("PROOF_016_023_mvcc_toctou_TTrace_1777795814.bin", TRUE)
\*
\*=============================================================================
\*

---- MODULE PROOF_016_023_mvcc_toctou_TETrace ----
EXTENDS TLC, PROOF_016_023_mvcc_toctou

trace == 
    <<
    ([phase |-> [T1 |-> "idle", T2 |-> "idle"],committed |-> {},writeSet |-> [T1 |-> {}, T2 |-> {}],pending |-> [T1 |-> {}, T2 |-> {}],readSet |-> [T1 |-> {}, T2 |-> {}],waitFor |-> [T1 |-> {}, T2 |-> {}],ts |-> [T1 |-> "T1", T2 |-> "T2"]]),
    ([phase |-> [T1 |-> "checked", T2 |-> "idle"],committed |-> {},writeSet |-> [T1 |-> {}, T2 |-> {}],pending |-> [T1 |-> {"T2"}, T2 |-> {}],readSet |-> [T1 |-> {}, T2 |-> {}],waitFor |-> [T1 |-> {}, T2 |-> {}],ts |-> [T1 |-> "T1", T2 |-> "T2"]]),
    ([phase |-> [T1 |-> "checked", T2 |-> "checked"],committed |-> {},writeSet |-> [T1 |-> {}, T2 |-> {}],pending |-> [T1 |-> {"T2"}, T2 |-> {"T1"}],readSet |-> [T1 |-> {}, T2 |-> {}],waitFor |-> [T1 |-> {}, T2 |-> {}],ts |-> [T1 |-> "T1", T2 |-> "T2"]]),
    ([phase |-> [T1 |-> "idle", T2 |-> "checked"],committed |-> {},writeSet |-> [T1 |-> {}, T2 |-> {}],pending |-> [T1 |-> {}, T2 |-> {"T1"}],readSet |-> [T1 |-> {}, T2 |-> {}],waitFor |-> [T1 |-> {"T2"}, T2 |-> {}],ts |-> [T1 |-> "T1", T2 |-> "T2"]]),
    ([phase |-> [T1 |-> "idle", T2 |-> "idle"],committed |-> {},writeSet |-> [T1 |-> {}, T2 |-> {}],pending |-> [T1 |-> {}, T2 |-> {}],readSet |-> [T1 |-> {}, T2 |-> {}],waitFor |-> [T1 |-> {"T2"}, T2 |-> {"T1"}],ts |-> [T1 |-> "T1", T2 |-> "T2"]])
    >>
----


=============================================================================

---- CONFIG PROOF_016_023_mvcc_toctou_TTrace_1777795814 ----
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
\* Generated on Sun May 03 16:10:15 CST 2026