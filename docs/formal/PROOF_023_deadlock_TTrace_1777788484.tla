---- MODULE PROOF_023_deadlock_TTrace_1777788484 ----
EXTENDS Sequences, TLCExt, Toolbox, Naturals, TLC, PROOF_023_deadlock

_expression ==
    LET PROOF_023_deadlock_TEExpression == INSTANCE PROOF_023_deadlock_TEExpression
    IN PROOF_023_deadlock_TEExpression!expression
----

_trace ==
    LET PROOF_023_deadlock_TETrace == INSTANCE PROOF_023_deadlock_TETrace
    IN PROOF_023_deadlock_TETrace!trace
----

_inv ==
    ~(
        TLCGet("level") = Len(_TETrace)
        /\
        txnHoldLocks = ((T1 :> {K1} @@ T2 :> {K2}))
        /\
        txnWaitFor = ((T1 :> T2 @@ T2 :> T1))
    )
----

_init ==
    /\ txnHoldLocks = _TETrace[1].txnHoldLocks
    /\ txnWaitFor = _TETrace[1].txnWaitFor
----

_next ==
    /\ \E i,j \in DOMAIN _TETrace:
        /\ \/ /\ j = i + 1
              /\ i = TLCGet("level")
        /\ txnHoldLocks  = _TETrace[i].txnHoldLocks
        /\ txnHoldLocks' = _TETrace[j].txnHoldLocks
        /\ txnWaitFor  = _TETrace[i].txnWaitFor
        /\ txnWaitFor' = _TETrace[j].txnWaitFor

\* Uncomment the ASSUME below to write the states of the error trace
\* to the given file in Json format. Note that you can pass any tuple
\* to `JsonSerialize`. For example, a sub-sequence of _TETrace.
    \* ASSUME
    \*     LET J == INSTANCE Json
    \*         IN J!JsonSerialize("PROOF_023_deadlock_TTrace_1777788484.json", _TETrace)

=============================================================================

 Note that you can extract this module `PROOF_023_deadlock_TEExpression`
  to a dedicated file to reuse `expression` (the module in the 
  dedicated `PROOF_023_deadlock_TEExpression.tla` file takes precedence 
  over the module `PROOF_023_deadlock_TEExpression` below).

---- MODULE PROOF_023_deadlock_TEExpression ----
EXTENDS Sequences, TLCExt, Toolbox, Naturals, TLC, PROOF_023_deadlock

expression == 
    [
        \* To hide variables of the `PROOF_023_deadlock` spec from the error trace,
        \* remove the variables below.  The trace will be written in the order
        \* of the fields of this record.
        txnHoldLocks |-> txnHoldLocks
        ,txnWaitFor |-> txnWaitFor
        
        \* Put additional constant-, state-, and action-level expressions here:
        \* ,_stateNumber |-> _TEPosition
        \* ,_txnHoldLocksUnchanged |-> txnHoldLocks = txnHoldLocks'
        
        \* Format the `txnHoldLocks` variable as Json value.
        \* ,_txnHoldLocksJson |->
        \*     LET J == INSTANCE Json
        \*     IN J!ToJson(txnHoldLocks)
        
        \* Lastly, you may build expressions over arbitrary sets of states by
        \* leveraging the _TETrace operator.  For example, this is how to
        \* count the number of times a spec variable changed up to the current
        \* state in the trace.
        \* ,_txnHoldLocksModCount |->
        \*     LET F[s \in DOMAIN _TETrace] ==
        \*         IF s = 1 THEN 0
        \*         ELSE IF _TETrace[s].txnHoldLocks # _TETrace[s-1].txnHoldLocks
        \*             THEN 1 + F[s-1] ELSE F[s-1]
        \*     IN F[_TEPosition - 1]
    ]

=============================================================================



Parsing and semantic processing can take forever if the trace below is long.
 In this case, it is advised to uncomment the module below to deserialize the
 trace from a generated binary file.

\*
\*---- MODULE PROOF_023_deadlock_TETrace ----
\*EXTENDS IOUtils, TLC, PROOF_023_deadlock
\*
\*trace == IODeserialize("PROOF_023_deadlock_TTrace_1777788484.bin", TRUE)
\*
\*=============================================================================
\*

---- MODULE PROOF_023_deadlock_TETrace ----
EXTENDS TLC, PROOF_023_deadlock

trace == 
    <<
    ([txnHoldLocks |-> (T1 :> {} @@ T2 :> {}),txnWaitFor |-> (T1 :> "none" @@ T2 :> "none")]),
    ([txnHoldLocks |-> (T1 :> {} @@ T2 :> {K2}),txnWaitFor |-> (T1 :> "none" @@ T2 :> "none")]),
    ([txnHoldLocks |-> (T1 :> {} @@ T2 :> {K2}),txnWaitFor |-> (T1 :> T2 @@ T2 :> "none")]),
    ([txnHoldLocks |-> (T1 :> {K1} @@ T2 :> {K2}),txnWaitFor |-> (T1 :> T2 @@ T2 :> "none")]),
    ([txnHoldLocks |-> (T1 :> {K1} @@ T2 :> {K2}),txnWaitFor |-> (T1 :> T2 @@ T2 :> T1)])
    >>
----


=============================================================================

---- CONFIG PROOF_023_deadlock_TTrace_1777788484 ----
CONSTANTS
    T1 = T1
    T2 = T2
    K1 = K1
    K2 = K2
    T1 = T1
    K2 = K2
    K1 = K1
    T2 = T2

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
\* Generated on Sun May 03 14:08:05 CST 2026