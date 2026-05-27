//! 培训记录追踪
//!
//! 追踪谁、何时、学了什么、正确率等信息。
//! 支持多学员、多课程的学习记录管理。

use crate::tutor::types::{TrainingRecord, TrainingStatus, TutorQuery, TutorResponse};
use std::collections::HashMap;

/// 培训记录追踪器
///
/// 管理所有学员的培训记录，支持：
/// - 创建培训课程
/// - 记录问答活动
/// - 查询学员进度
/// - 生成培训报告
#[derive(Debug, Clone)]
pub struct TrainingTracker {
    /// 所有培训记录（record_id -> TrainingRecord）
    records: HashMap<String, TrainingRecord>,
    /// 学员索引（trainee_id -> [record_id]）
    trainee_index: HashMap<String, Vec<String>>,
}

impl Default for TrainingTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl TrainingTracker {
    /// 创建新的培训记录追踪器
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
            trainee_index: HashMap::new(),
        }
    }

    /// 为学员创建新的培训记录
    ///
    /// # Arguments
    /// * `trainee_id` - 学员 ID
    /// * `topic` - 培训主题
    ///
    /// # Returns
    /// 创建的培训记录
    pub fn create_record(&mut self, trainee_id: &str, topic: &str) -> TrainingRecord {
        let record = TrainingRecord::new(trainee_id, topic);
        let record_id = record.record_id.clone();
        self.records.insert(record_id.clone(), record);
        self.trainee_index
            .entry(trainee_id.to_string())
            .or_default()
            .push(record_id.clone());

        self.records.get(&record_id).cloned().unwrap()
    }

    /// 记录一次问答活动到指定培训记录
    pub fn record_activity(
        &mut self,
        record_id: &str,
        is_correct: bool,
        knowledge_id: &str,
    ) -> Option<()> {
        let record = self.records.get_mut(record_id)?;
        record.record_answer(is_correct, knowledge_id);
        Some(())
    }

    /// 自动记录查询（将问答映射到学员最近的活动记录）
    pub fn record_query(&mut self, query: &TutorQuery, response: &TutorResponse) {
        let trainee_id = &query.user_id;

        // 获取学员最新的活动记录
        let record_id = self
            .trainee_index
            .get(trainee_id)
            .and_then(|ids| ids.last().cloned());

        if let Some(rid) = record_id {
            // 检查回答是否包含引用（认为有引用 = 正确/有效）
            let is_correct = !response.citations.is_empty() && response.is_grounded;
            let knowledge_id = response.knowledge_ids.first().cloned().unwrap_or_default();

            self.record_activity(&rid, is_correct, &knowledge_id);
        }
    }

    /// 完成一个培训记录
    pub fn complete_record(&mut self, record_id: &str, passing_threshold: f64) -> Option<()> {
        let record = self.records.get_mut(record_id)?;
        record.complete(passing_threshold);
        Some(())
    }

    /// 获取学员的所有培训记录
    pub fn get_trainee_records(&self, trainee_id: &str) -> Vec<&TrainingRecord> {
        self.trainee_index
            .get(trainee_id)
            .map(|ids| ids.iter().filter_map(|id| self.records.get(id)).collect())
            .unwrap_or_default()
    }

    /// 获取单个培训记录
    pub fn get_record(&self, record_id: &str) -> Option<&TrainingRecord> {
        self.records.get(record_id)
    }

    /// 获取所有学员 ID
    pub fn all_trainees(&self) -> Vec<&String> {
        self.trainee_index.keys().collect()
    }

    /// 获取所有培训记录
    pub fn all_records(&self) -> Vec<&TrainingRecord> {
        self.records.values().collect()
    }

    /// 获取指定学员的统计信息
    pub fn trainee_stats(&self, trainee_id: &str) -> TraineeStats {
        let records = self.get_trainee_records(trainee_id);
        let total_sessions = records.len();
        let total_questions: u32 = records.iter().map(|r| r.question_count).sum();
        let total_correct: u32 = records.iter().map(|r| r.correct_count).sum();
        let completed: usize = records
            .iter()
            .filter(|r| r.status == TrainingStatus::Completed)
            .count();
        let in_progress: usize = records
            .iter()
            .filter(|r| r.status == TrainingStatus::InProgress)
            .count();
        let failed: usize = records
            .iter()
            .filter(|r| r.status == TrainingStatus::Failed)
            .count();

        let overall_mastery = if total_questions > 0 {
            total_correct as f64 / total_questions as f64
        } else {
            0.0
        };

        TraineeStats {
            trainee_id: trainee_id.to_string(),
            total_sessions,
            total_questions,
            total_correct,
            completed_sessions: completed,
            in_progress_sessions: in_progress,
            failed_sessions: failed,
            overall_mastery,
        }
    }

    /// 清除所有记录（用于测试）
    pub fn clear(&mut self) {
        self.records.clear();
        self.trainee_index.clear();
    }
}

/// 学员统计信息
#[derive(Debug, Clone)]
pub struct TraineeStats {
    /// 学员 ID
    pub trainee_id: String,
    /// 培训课程总数
    pub total_sessions: usize,
    /// 总提问数
    pub total_questions: u32,
    /// 总正确数
    pub total_correct: u32,
    /// 已完成的课程数
    pub completed_sessions: usize,
    /// 进行中的课程数
    pub in_progress_sessions: usize,
    /// 未通过的课程数
    pub failed_sessions: usize,
    /// 总体掌握率
    pub overall_mastery: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // ── TrainingTracker creation tests ──

    #[test]
    fn test_tracker_creation() {
        let tracker = TrainingTracker::new();
        assert!(tracker.all_records().is_empty());
        assert!(tracker.all_trainees().is_empty());
    }

    // ── Create record tests ──

    #[test]
    fn test_create_record() {
        let mut tracker = TrainingTracker::new();
        let record = tracker.create_record("trainee-001", "GMP 基础知识");
        assert_eq!(record.trainee_id, "trainee-001");
        assert_eq!(record.topic, "GMP 基础知识");
        assert_eq!(record.status, TrainingStatus::InProgress);
        assert_eq!(tracker.all_records().len(), 1);
    }

    #[test]
    fn test_create_multiple_records() {
        let mut tracker = TrainingTracker::new();
        tracker.create_record("trainee-001", "GMP 基础知识");
        tracker.create_record("trainee-001", "验证专题");
        tracker.create_record("trainee-002", "质量体系");
        assert_eq!(tracker.all_records().len(), 3);
    }

    // ── Get records tests ──

    #[test]
    fn test_get_trainee_records() {
        let mut tracker = TrainingTracker::new();
        tracker.create_record("trainee-001", "GMP 基础知识");
        tracker.create_record("trainee-001", "验证专题");
        tracker.create_record("trainee-002", "质量体系");

        let records_1 = tracker.get_trainee_records("trainee-001");
        assert_eq!(records_1.len(), 2);

        let records_2 = tracker.get_trainee_records("trainee-002");
        assert_eq!(records_2.len(), 1);

        let records_3 = tracker.get_trainee_records("trainee-nonexistent");
        assert!(records_3.is_empty());
    }

    #[test]
    fn test_get_record() {
        let mut tracker = TrainingTracker::new();
        let record = tracker.create_record("trainee-001", "GMP 基础知识");
        let record_id = record.record_id.clone();

        let found = tracker.get_record(&record_id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().trainee_id, "trainee-001");

        let not_found = tracker.get_record("nonexistent-id");
        assert!(not_found.is_none());
    }

    // ── Record activity tests ──

    #[test]
    fn test_record_activity_correct() {
        let mut tracker = TrainingTracker::new();
        let record = tracker.create_record("trainee-001", "GMP 基础知识");
        let record_id = record.record_id.clone();

        tracker.record_activity(&record_id, true, "k-001");

        let updated = tracker.get_record(&record_id).unwrap();
        assert_eq!(updated.question_count, 1);
        assert_eq!(updated.correct_count, 1);
        assert!((updated.mastery_rate - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_record_activity_wrong() {
        let mut tracker = TrainingTracker::new();
        let record = tracker.create_record("trainee-001", "GMP 基础知识");
        let record_id = record.record_id.clone();

        tracker.record_activity(&record_id, false, "k-001");
        tracker.record_activity(&record_id, false, "k-002");

        let updated = tracker.get_record(&record_id).unwrap();
        assert_eq!(updated.question_count, 2);
        assert_eq!(updated.correct_count, 0);
        assert!((updated.mastery_rate - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_record_activity_nonexistent_record() {
        let mut tracker = TrainingTracker::new();
        let result = tracker.record_activity("nonexistent", true, "k-001");
        assert!(result.is_none());
    }

    #[test]
    fn test_record_activity_tracks_knowledge_ids() {
        let mut tracker = TrainingTracker::new();
        let record = tracker.create_record("trainee-001", "GMP 基础知识");
        let record_id = record.record_id.clone();

        tracker.record_activity(&record_id, true, "k-001");
        tracker.record_activity(&record_id, true, "k-002");

        let updated = tracker.get_record(&record_id).unwrap();
        assert!(updated.knowledge_ids.contains(&"k-001".to_string()));
        assert!(updated.knowledge_ids.contains(&"k-002".to_string()));
    }

    // ── Record query tests ──

    #[test]
    fn test_record_query_updates_record() {
        let mut tracker = TrainingTracker::new();
        tracker.create_record("trainee-001", "GMP 基础知识");

        let query = TutorQuery::new("trainee-001", "生产过程控制？");
        let response = TutorResponse {
            query_id: query.query_id.clone(),
            answer: "测试回答".to_string(),
            citations: vec!["21 CFR Part 211".to_string()],
            knowledge_ids: vec!["k-001".to_string()],
            relevant_regulations: Vec::new(),
            is_grounded: true,
            model: None,
            confidence: 0.9,
            processing_time_ms: 100,
            timestamp: Utc::now(),
        };

        tracker.record_query(&query, &response);

        let records = tracker.get_trainee_records("trainee-001");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].question_count, 1);
        assert_eq!(records[0].correct_count, 1);
    }

    #[test]
    fn test_record_query_no_record() {
        let mut tracker = TrainingTracker::new();
        let query = TutorQuery::new("trainee-001", "测试？");
        let response = TutorResponse::cannot_answer("q-001");

        // Should not panic when there's no record for the trainee
        tracker.record_query(&query, &response);
        assert!(tracker.get_trainee_records("trainee-001").is_empty());
    }

    // ── Complete record tests ──

    #[test]
    fn test_complete_record_pass() {
        let mut tracker = TrainingTracker::new();
        let record = tracker.create_record("trainee-001", "GMP 基础知识");
        let record_id = record.record_id.clone();

        tracker.record_activity(&record_id, true, "k-001");
        tracker.record_activity(&record_id, true, "k-002");
        tracker.complete_record(&record_id, 0.8);

        let updated = tracker.get_record(&record_id).unwrap();
        assert_eq!(updated.status, TrainingStatus::Completed);
    }

    #[test]
    fn test_complete_record_fail() {
        let mut tracker = TrainingTracker::new();
        let record = tracker.create_record("trainee-001", "GMP 基础知识");
        let record_id = record.record_id.clone();

        tracker.record_activity(&record_id, false, "k-001");
        tracker.record_activity(&record_id, false, "k-002");
        tracker.complete_record(&record_id, 0.8);

        let updated = tracker.get_record(&record_id).unwrap();
        assert_eq!(updated.status, TrainingStatus::Failed);
    }

    #[test]
    fn test_complete_nonexistent_record() {
        let mut tracker = TrainingTracker::new();
        let result = tracker.complete_record("nonexistent", 0.8);
        assert!(result.is_none());
    }

    // ── Trainee stats tests ──

    #[test]
    fn test_trainee_stats_empty() {
        let tracker = TrainingTracker::new();
        let stats = tracker.trainee_stats("trainee-001");
        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.total_questions, 0);
        assert!((stats.overall_mastery - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_trainee_stats_with_records() {
        let mut tracker = TrainingTracker::new();
        let r1 = tracker.create_record("trainee-001", "GMP 基础知识");
        let r2 = tracker.create_record("trainee-001", "验证专题");

        tracker.record_activity(&r1.record_id, true, "k-001");
        tracker.record_activity(&r1.record_id, true, "k-002");
        tracker.record_activity(&r2.record_id, false, "k-003");

        let stats = tracker.trainee_stats("trainee-001");
        assert_eq!(stats.total_sessions, 2);
        assert_eq!(stats.total_questions, 3);
        assert_eq!(stats.total_correct, 2);
        assert!((stats.overall_mastery - 2.0 / 3.0).abs() < f64::EPSILON);
    }

    // ── All trainees test ──

    #[test]
    fn test_all_trainees() {
        let mut tracker = TrainingTracker::new();
        tracker.create_record("trainee-001", "GMP");
        tracker.create_record("trainee-002", "验证");
        tracker.create_record("trainee-001", "质量");

        let trainees = tracker.all_trainees();
        assert_eq!(trainees.len(), 2);
        assert!(trainees.contains(&&"trainee-001".to_string()));
        assert!(trainees.contains(&&"trainee-002".to_string()));
    }

    // ── Clear test ──

    #[test]
    fn test_clear() {
        let mut tracker = TrainingTracker::new();
        tracker.create_record("trainee-001", "GMP");
        assert_eq!(tracker.all_records().len(), 1);

        tracker.clear();
        assert!(tracker.all_records().is_empty());
        assert!(tracker.all_trainees().is_empty());
    }
}
