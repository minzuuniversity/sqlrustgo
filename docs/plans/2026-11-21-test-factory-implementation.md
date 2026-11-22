# Test Factory Independent Contract Domain Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 建立与常规回归隔离、持续实跑、可精确分类 RED 的契约测试运行域。

**Architecture:** 独立 Cargo manifest 显式运行契约 case；Test Factory 使用执行状态与契约状态双轴分类。正式 DAG、ADR、Issue 和测试源码保持唯一权威，本域只生成可验证收据。

**Tech Stack:** Rust/Cargo、Python Test Factory、Gitea Actions、JSON Schema、Bash gates。

---

详细任务、路径、正反测试、退出码、回滚和审核角色见 `docs/releases/v4.1.0/refactoring/test-factory-design/09_IMPLEMENTATION_TASK_DAG.jsonl`。执行顺序严格为 TF-01 → TF-02 → TF-03 → TF-04/TF-05 → TF-06 → TF-07。

本计划未获实施授权；任何任务开始前必须由 Owner 填写预算、确认固定 base，并满足对应 HUMAN_GATE。
