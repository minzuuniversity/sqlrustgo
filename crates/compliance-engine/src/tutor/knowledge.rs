//! GMP 法规知识库
//!
//! 内置 GMP 法规知识库，包含：
//! - 21 CFR Part 210/211 (美国 cGMP)
//! - EU GMP Annex 11 (计算机化系统)
//! - ICH Q7 (原料药 GMP)
//! - WHO GMP (世界卫生组织)
//! - 中国 GMP (NMPA)
//! - PIC/S GMP
//!
//! 提供关键词检索、分类过滤和相关性排序功能。

use crate::tutor::types::Regulation;
use std::collections::HashMap;

/// 知识检索器
///
/// 管理 GMP 法规知识库，支持自然语言查询和精确匹配检索。
#[derive(Debug, Clone)]
pub struct KnowledgeRetriever {
    /// 法规库（id -> Regulation）
    regulations: HashMap<String, Regulation>,
    /// 分类索引（category -> [regulation_id]）
    category_index: HashMap<String, Vec<String>>,
}

impl Default for KnowledgeRetriever {
    fn default() -> Self {
        Self::new()
    }
}

impl KnowledgeRetriever {
    /// 创建新的知识检索器并加载内置法规
    pub fn new() -> Self {
        let mut retriever = Self {
            regulations: HashMap::new(),
            category_index: HashMap::new(),
        };
        retriever.load_builtin_regulations();
        retriever
    }

    /// 加载内置 GMP 法规
    fn load_builtin_regulations(&mut self) {
        let regulations = builtin_gmp_regulations();
        for reg in regulations {
            self.add_regulation(reg);
        }
    }

    /// 添加一条法规到知识库
    pub fn add_regulation(&mut self, regulation: Regulation) {
        let id = regulation.id.clone();
        let category = regulation.category.clone();
        self.regulations.insert(id.clone(), regulation);
        self.category_index.entry(category).or_default().push(id);
    }

    /// 获取所有法规
    pub fn all_regulations(&self) -> Vec<&Regulation> {
        self.regulations.values().collect()
    }

    /// 根据 ID 获取法规
    pub fn get_regulation(&self, id: &str) -> Option<&Regulation> {
        self.regulations.get(id)
    }

    /// 根据分类获取法规
    pub fn get_by_category(&self, category: &str) -> Vec<&Regulation> {
        self.category_index
            .get(category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.regulations.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 获取所有分类
    pub fn categories(&self) -> Vec<String> {
        let mut cats: Vec<String> = self.category_index.keys().cloned().collect();
        cats.sort();
        cats
    }

    /// 搜索法规（关键词匹配 + 相关性排序）
    ///
    /// 返回匹配的法规列表，按相关性得分降序排列。
    pub fn search(&self, query: &str, top_k: usize) -> Vec<&Regulation> {
        let query_lower = query.to_lowercase();
        let mut scored: Vec<(&Regulation, f64)> = self
            .regulations
            .values()
            .filter(|reg| reg.matches(&query_lower))
            .map(|reg| {
                let score = reg.relevance_score(&query_lower);
                (reg, score)
            })
            .collect();

        // 按相关性得分降序排列
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scored.into_iter().take(top_k).map(|(reg, _)| reg).collect()
    }

    /// 带分类过滤的搜索
    pub fn search_with_category(
        &self,
        query: &str,
        category: &str,
        top_k: usize,
    ) -> Vec<&Regulation> {
        let query_lower = query.to_lowercase();
        let category_lower = category.to_lowercase();

        let mut scored: Vec<(&Regulation, f64)> = self
            .regulations
            .values()
            .filter(|reg| {
                reg.category.to_lowercase() == category_lower && reg.matches(&query_lower)
            })
            .map(|reg| {
                let score = reg.relevance_score(&query_lower);
                (reg, score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scored.into_iter().take(top_k).map(|(reg, _)| reg).collect()
    }

    /// 统计知识库信息
    pub fn stats(&self) -> KnowledgeStats {
        let total = self.regulations.len();
        let categories: HashMap<String, usize> = self
            .category_index
            .iter()
            .map(|(cat, ids)| (cat.clone(), ids.len()))
            .collect();

        KnowledgeStats {
            total_regulations: total,
            categories,
        }
    }
}

/// 知识库统计信息
#[derive(Debug, Clone)]
pub struct KnowledgeStats {
    pub total_regulations: usize,
    pub categories: HashMap<String, usize>,
}

/// 构建内置 GMP 法规知识库（至少 5 条核心法规）
pub fn builtin_gmp_regulations() -> Vec<Regulation> {
    vec![
        // 1. 21 CFR Part 211 — 美国 cGMP 成品药
        Regulation::new(
            "cfr-211-110",
            "21 CFR 211.110 生产和过程控制",
            "21 CFR Part 211 — cGMP for Finished Pharmaceuticals",
            "生产过程应进行控制以确保产品质量符合既定规格。所有生产操作应按照书面程序进行，并记录关键工艺参数。任何偏差均应记录并调查。生产过程中的取样和检测应按照书面程序进行，检测结果应记录。",
            "生产",
            vec!["生产", "过程控制", "工艺参数", "偏差", "取样", "cGMP", "211"],
            "2024",
        ),
        // 2. 21 CFR Part 211 — 实验室控制
        Regulation::new(
            "cfr-211-160",
            "21 CFR 211.160 实验室控制",
            "21 CFR Part 211 — cGMP for Finished Pharmaceuticals",
            "实验室控制应包括建立科学和合理的规格、标准、取样计划和分析程序，以确保原料、包装材料、中间体和成品符合既定规格。实验室记录应包含所有试验数据的完整记录。所有检验方法应经过验证。",
            "质量",
            vec!["实验室", "检测", "质量控制", "规格", "验证", "取样"],
            "2024",
        ),
        // 3. 21 CFR Part 211 — 记录与报告
        Regulation::new(
            "cfr-211-180",
            "21 CFR 211.180 记录与报告要求",
            "21 CFR Part 211 — cGMP for Finished Pharmaceuticals",
            "所有生产和控制记录应保存至少一年后成品有效期后一年，以较长者为准。记录应清晰、完整，并按照书面程序进行审核。任何记录的错误应采用划线更正的方式修改，不得涂改。记录应包含签名和日期。",
            "文件管理",
            vec!["记录", "报告", "文件管理", "签名", "数据完整性", "保存"],
            "2024",
        ),
        // 4. 21 CFR Part 211 — 设备
        Regulation::new(
            "cfr-211-63",
            "21 CFR 211.63 设备设计、尺寸和位置",
            "21 CFR Part 211 — cGMP for Finished Pharmaceuticals",
            "设备应设计为便于清洁、维护和操作。设备表面不应与组分发生反应、添加或吸附而改变产品质量。设备应按照使用用途合理放置。建立设备使用、清洁和维护的书面程序。",
            "设备",
            vec!["设备", "清洁", "维护", "设计", "校准"],
            "2024",
        ),
        // 5. EU GMP Annex 11 — 计算机化系统
        Regulation::new(
            "eu-annex11-1",
            "EU GMP Annex 11 计算机化系统验证",
            "EU GMP Annex 11 — Computerised Systems",
            "计算机化系统替代人工操作时，不应因此降低产品质量、过程控制或质量保证。系统应经过验证，验证应考虑预期用途、系统复杂性和可靠性。系统应具备数据完整性保护机制，包括审计追踪、用户权限控制和电子签名。",
            "验证",
            vec!["计算机化", "验证", "数据完整性", "审计追踪", "电子签名", "权限"],
            "2023",
        ),
        // 6. EU GMP Annex 11 — 数据完整性
        Regulation::new(
            "eu-annex11-7",
            "EU GMP Annex 11 数据完整性要求",
            "EU GMP Annex 11 — Computerised Systems",
            "应确保数据的完整性和安全性。系统应能防止数据丢失或损坏。数据输入应由授权人员进行，并有适当的复核机制。关键数据的修改应有审计追踪记录。系统应生成记录所有关键活动的审计追踪。",
            "验证",
            vec!["数据完整性", "审计追踪", "安全性", "数据保护", "ALCOA"],
            "2023",
        ),
        // 7. ICH Q7 — 原料药 GMP
        Regulation::new(
            "ich-q7-2",
            "ICH Q7 原料药 GMP 质量管理",
            "ICH Q7 — GMP for Active Pharmaceutical Ingredients",
            "原料药生产应建立质量管理体系，包括质量方针、质量目标、质量计划和持续改进。应有独立的 QA 部门负责质量保证活动。所有偏差应记录和调查。应建立变更控制程序。定期进行自检或质量审计。",
            "质量",
            vec!["原料药", "质量管理", "QA", "偏差", "变更控制", "自检", "ICH"],
            "2022",
        ),
        // 8. ICH Q7 — 验证
        Regulation::new(
            "ich-q7-12",
            "ICH Q7 原料药验证要求",
            "ICH Q7 — GMP for Active Pharmaceutical Ingredients",
            "验证应包括工艺验证、清洁验证和分析方法验证。工艺验证应证明工艺能够持续生产符合质量要求的原料药。清洁验证应证明设备清洁程序的有效性。分析方法验证应证明方法适用于预期用途。",
            "验证",
            vec!["验证", "工艺验证", "清洁验证", "分析方法", "确认"],
            "2022",
        ),
        // 9. WHO GMP — 人员
        Regulation::new(
            "who-gmp-2",
            "WHO GMP 人员培训要求",
            "WHO GMP — Main Principles for Pharmaceutical Products",
            "所有从事生产和质量控制的人员应接受基础培训和持续培训。培训内容应包括 GMP 原则、标准操作程序（SOP）、卫生规范和岗位技能。培训效果应定期评估。培训记录应保存。关键岗位人员应具备相应的教育背景和工作经验。",
            "人员",
            vec!["培训", "人员", "SOP", "卫生", "资格", "WHO"],
            "2023",
        ),
        // 10. 中国 GMP (NMPA) — 生产管理
        Regulation::new(
            "nmpa-gmp-186",
            "中国 GMP 生产管理要求（第 186 条）",
            "中国 GMP (2010年修订) — NMPA",
            "每批产品应按照批准的工艺规程和操作规程进行生产。生产和包装操作应有记录。生产过程中应避免交叉污染。关键工艺参数应在规定范围内。生产记录应经审核。包装材料应按规定管理。",
            "生产",
            vec!["生产管理", "工艺规程", "交叉污染", "包装", "中国GMP", "NMPA"],
            "2019",
        ),
        // 11. PIC/S GMP — 产品质量回顾
        Regulation::new(
            "pics-gmp-1",
            "PIC/S GMP 产品质量回顾分析",
            "PIC/S GMP — Pharmaceutical Inspection Co-operation Scheme",
            "应定期对产品进行质量回顾分析，以确保工艺持续处于受控状态。回顾应包括关键工艺参数、检测结果、偏差、变更、稳定性数据和投诉等信息。回顾结果应形成书面报告，并采取必要的纠正和预防措施。",
            "质量",
            vec!["质量回顾", "持续改进", "稳定性", "投诉", "CAPA", "PIC/S"],
            "2023",
        ),
        // 12. 21 CFR Part 210 — 定义与范围
        Regulation::new(
            "cfr-210-1",
            "21 CFR Part 210 cGMP 定义与范围",
            "21 CFR Part 210 — cGMP for Drug Products",
            "cGMP (现行良好生产规范) 是确保药品安全、纯度和质量的最低标准。本章规定适用于所有人用药品的生产、加工、包装和储存。cGMP 要求建立全面的质量管理系统，包括设施、设备、人员、材料和过程的控制。",
            "生产",
            vec!["cGMP", "定义", "范围", "药品质量", "安全"],
            "2024",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── KnowledgeRetriever tests ──

    #[test]
    fn test_knowledge_retriever_creation() {
        let kr = KnowledgeRetriever::new();
        let stats = kr.stats();
        assert!(stats.total_regulations >= 5, "至少需要 5 条核心法规");
        assert!(!stats.categories.is_empty());
    }

    #[test]
    fn test_search_by_keyword() {
        let kr = KnowledgeRetriever::new();
        let results = kr.search("计算机化系统验证", 5);
        assert!(!results.is_empty(), "应找到计算机化系统相关法规");
        let has_annex11 = results.iter().any(|r| r.id.starts_with("eu-annex11"));
        assert!(has_annex11, "应包含 EU GMP Annex 11");
    }

    #[test]
    fn test_search_by_multiple_keywords() {
        let kr = KnowledgeRetriever::new();
        let results = kr.search("数据完整性 审计追踪", 5);
        assert!(!results.is_empty(), "应找到数据完整性相关法规");
    }

    #[test]
    fn test_search_empty_query() {
        let kr = KnowledgeRetriever::new();
        let results = kr.search("", 5);
        assert!(results.is_empty(), "空查询应返回空结果");
    }

    #[test]
    fn test_search_top_k() {
        let kr = KnowledgeRetriever::new();
        let results = kr.search("生产", 3);
        assert!(results.len() <= 3, "top_k 限制应生效");
    }

    #[test]
    fn test_search_no_match() {
        let kr = KnowledgeRetriever::new();
        let results = kr.search("zzz_not_a_gmp_term_xyz", 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_get_by_category() {
        let kr = KnowledgeRetriever::new();
        let validation = kr.get_by_category("验证");
        assert!(!validation.is_empty(), "验证类应有法规");
        let all_cats = kr.categories();
        assert!(all_cats.contains(&"验证".to_string()));
    }

    #[test]
    fn test_get_by_nonexistent_category() {
        let kr = KnowledgeRetriever::new();
        let results = kr.get_by_category("nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_get_regulation_by_id() {
        let kr = KnowledgeRetriever::new();
        let reg = kr.get_regulation("cfr-211-110");
        assert!(reg.is_some());
        assert_eq!(
            reg.unwrap().source,
            "21 CFR Part 211 — cGMP for Finished Pharmaceuticals"
        );
    }

    #[test]
    fn test_get_nonexistent_regulation() {
        let kr = KnowledgeRetriever::new();
        let reg = kr.get_regulation("nonexistent-id");
        assert!(reg.is_none());
    }

    #[test]
    fn test_all_regulations_count() {
        let kr = KnowledgeRetriever::new();
        let all = kr.all_regulations();
        assert!(all.len() >= 5, "至少应有 5 条法规");
    }

    #[test]
    fn test_add_custom_regulation() {
        let mut kr = KnowledgeRetriever::new();
        let custom = Regulation::new(
            "custom-001",
            "自定义测试法规",
            "Custom Source",
            "测试内容",
            "自定义",
            vec!["测试"],
            "1.0",
        );
        kr.add_regulation(custom);
        let reg = kr.get_regulation("custom-001");
        assert!(reg.is_some());
    }

    #[test]
    fn test_search_with_category_filter() {
        let kr = KnowledgeRetriever::new();
        let results = kr.search_with_category("验证", "验证", 5);
        assert!(!results.is_empty());
        for reg in &results {
            assert_eq!(reg.category, "验证");
        }
    }

    #[test]
    fn test_categories_sorted() {
        let kr = KnowledgeRetriever::new();
        let cats = kr.categories();
        let mut sorted = cats.clone();
        sorted.sort();
        assert_eq!(cats, sorted);
    }

    #[test]
    fn test_stats() {
        let kr = KnowledgeRetriever::new();
        let stats = kr.stats();
        assert!(stats.total_regulations > 0);
        assert!(!stats.categories.is_empty());
        let total_from_cats: usize = stats.categories.values().sum();
        assert_eq!(total_from_cats, stats.total_regulations);
    }
}
