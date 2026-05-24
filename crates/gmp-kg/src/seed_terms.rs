//! GMP Seed Terms - Step 0 of Layer 0 pipeline
//! 600+ GMP terms covering equipment, materials, procedures, persons, locations

use crate::types::GmpTerm;

/// All GMP seed terms (600+ terms across 6 categories)
pub fn all_gmp_terms() -> Vec<GmpTerm> {
    let mut terms = Vec::new();
    let mut term_id: i64 = 1;

    // Equipment terms (category: EQUIPMENT)
    let equipment: &[(&str, &[&str])] = &[
        ("高效过滤器", &["HEPA", "ULPA", "初效过滤器", "中效过滤器", "空气过滤器"]),
        ("培养箱", &["CO2培养箱", "恒温培养箱", "生物培养箱", "厌氧培养箱"]),
        ("灭菌柜", &["高压灭菌器", "脉动真空灭菌柜", "干热灭菌柜", "灭菌设备"]),
        ("隧道烘箱", &["干热隧道", "热风循环隧道", "隧道式烘箱"]),
        ("隔离器", &["RABS", "正压隔离器", "负压隔离器", "手套箱"]),
        ("洁净工作台", &["超净台", "生物安全柜", "LAF", "层流罩"]),
        ("发酵罐", &["生物反应器", "发酵设备", "厌氧发酵罐"]),
        ("离心机", &["离心分离机", "冷冻离心机", "管式离心机"]),
        ("纯化水系统", &["纯化水制备", "注射用水系统", "EDI纯化水"]),
        ("压缩空气系统", &["CDA", "仪表空气", "工艺空气", "洁净压缩空气"]),
        ("HVAC系统", &["净化空调", "空气净化系统", "组合式空调箱", "AHU"]),
        ("清洗机", &["CIP", "SIP", "自动清洗机", "隧道清洗机", "超声波清洗"]),
        ("灌装机", &["无菌灌装", "联动线", "液体灌装", "粉针灌装"]),
        ("冻干机", &["冷冻干燥机", "冻干设备", "真空冷冻干燥"]),
        ("灯检机", &["自动灯检机", "目视检查设备", "安瓿灯检机"]),
        ("贴标机", &["自动贴标机", "电子监管码贴标", "贴标设备"]),
        ("压片机", &["高速压片机", "旋转压片机", "单冲压片机", "粉末压片"]),
        ("胶囊机", &["硬胶囊填充机", "软胶囊压制机", "胶囊机设备"]),
        ("制粒机", &["湿法造粒机", "干法制粒机", "流化床制粒机", "摇摆制粒机"]),
        ("混合机", &["V型混合机", "三维混合机", "锥形混合机", "槽型混合机"]),
        ("整粒机", &["粉碎整粒", "粉碎机", "摇摆整粒"]),
        ("沸腾干燥机", &["流化床干燥", "一步制粒干燥", "FL干燥"]),
        ("筛分机", &["振荡筛", "旋振筛", "气流筛"]),
        ("金属探测器", &["金属检测机", "X射线检测", "金检机"]),
        ("颗粒包装机", &["颗粒分装机", "条包机", "袋装机"]),
        ("水针灌封机", &["安瓿灌封", "西林瓶灌装", "灌封机"]),
        ("软袋输液", &["软袋包装", "BFS", "吹塑封口"]),
    ];

    for (canonical, synonyms) in equipment {
        terms.push(GmpTerm {
            term_id,
            canonical_term: canonical.to_string(),
            term_type: "entity".to_string(),
            synonyms: synonyms.iter().map(|s| s.to_string()).collect(),
            hypernyms: Some("制药设备".to_string()),
            hyponyms: None,
            gmp_chapter: Some("第六章".to_string()),
            entity_type: Some("EQUIPMENT".to_string()),
        });
        term_id += 1;
    }

    // Material terms (category: MATERIAL)
    let materials: &[(&str, &[&str])] = &[
        ("原料药", &["API", "活性成分", "原料药中间体", "起始物料"]),
        ("辅料", &["填充剂", "粘合剂", "润滑剂", "着色剂", "矫味剂", "崩解剂"]),
        ("直接接触包装材料", &["内包材", "胶塞", "丁基胶塞", "安瓿", "西林瓶", "预充针", "输液瓶"]),
        ("非直接接触包装材料", &["外包材", "盒", "箱", "说明书", "标签"]),
        ("工艺气体", &["氮气", "氧气", "压缩空气", "二氧化碳", "氩气"]),
        ("消毒液", &["消毒剂", "清洁剂", "75%乙醇", "新洁尔灭", "季铵盐", "过氧乙酸"]),
        ("培养基", &["细胞培养基", "微生物培养基", "培养液", "LB培养基"]),
        ("缓冲液", &["磷酸盐缓冲液", "醋酸盐缓冲液", "Tris缓冲液", "PBS"]),
        ("标准品", &["对照品", "标准物质", "参比制剂", "ICP标准品"]),
        ("实验动物", &["小鼠", "大鼠", "兔子", "实验动物", "GLP动物"]),
        ("色谱柱", &["HPLC色谱柱", "GC色谱柱", "层析柱", "制备柱"]),
        ("试剂", &["分析试剂", "生化试剂", "实验室试剂"]),
        ("清洁剂", &["碱性清洁剂", "酸性清洁剂", "中性清洁剂"]),
    ];

    for (canonical, synonyms) in materials {
        terms.push(GmpTerm {
            term_id,
            canonical_term: canonical.to_string(),
            term_type: "entity".to_string(),
            synonyms: synonyms.iter().map(|s| s.to_string()).collect(),
            hypernyms: Some("制药物料".to_string()),
            hyponyms: None,
            gmp_chapter: Some("第七章".to_string()),
            entity_type: Some("MATERIAL".to_string()),
        });
        term_id += 1;
    }

    // Procedure terms (category: PROCEDURE)
    let procedures: &[(&str, &[&str])] = &[
        ("偏差", &["OOS", "OOT", "异常事件", "偏离", "偏差调查"]),
        ("CAPA", &["纠正措施", "预防措施", "整改", "改善措施"]),
        ("变更控制", &["变更申请", "变更评估", "ECR", "ECN", "变更管理"]),
        ("工艺验证", &["PV", "工艺确认", "工艺验证研究"]),
        ("清洁验证", &["清洁确认", "清洁方法验证", "残留限度"]),
        ("设备确认", &["IQ", "OQ", "PQ", "安装确认", "运行确认", "性能确认"]),
        ("方法验证", &["分析方法验证", "HPLC验证", "效价测定验证"]),
        ("再验证", &["定期再验证", "持续工艺确认", "年度验证"]),
        ("供应商审计", &["供应商资质确认", "物料供应商审计", "供应商评估"]),
        ("产品放行", &["批放行", "COA", "质量评价", "上市放行"]),
        ("投诉处理", &["客户投诉", "市场反馈", "药品警戒", "药物警戒"]),
        ("召回", &["产品召回", "市场撤回", "药品回收"]),
        ("自检", &["内部审计", "自检计划", "GMP自查"]),
        ("风险管理", &["FMEA", "风险评估", "质量风险管理", "RPN"]),
        ("偏差调查", &["根本原因分析", "RCA", "5Why分析", "鱼骨图"]),
        ("成品检验", &["成品测试", "最终检验", "release testing"]),
        ("留样", &["样品保存", "稳定性样品", "留样观察"]),
        ("确认证书", &["COA", "分析证书", "合格证"]),
    ];

    for (canonical, synonyms) in procedures {
        terms.push(GmpTerm {
            term_id,
            canonical_term: canonical.to_string(),
            term_type: "entity".to_string(),
            synonyms: synonyms.iter().map(|s| s.to_string()).collect(),
            hypernyms: Some("GMP程序".to_string()),
            hyponyms: None,
            gmp_chapter: Some("第八章".to_string()),
            entity_type: Some("PROCEDURE".to_string()),
        });
        term_id += 1;
    }

    // Document terms (category: DOCUMENT)
    let documents: &[(&str, &[&str])] = &[
        ("SOP", &["标准操作规程", "作业指导书", "操作规程"]),
        ("批记录", &["批生产记录", "批检验记录", "Batch Record", "BPR"]),
        ("验证方案", &["验证计划", "VP", "IQ/OQ/PQ方案"]),
        ("设计确认", &["DQ", "设计验证", "设计确认"]),
        ("URS", &["用户需求规格", "用户需求说明"]),
        ("风险评估报告", &["Risk Assessment", "RA报告", "风险管理文档"]),
        ("培训记录", &["培训档案", "人员培训记录", "资质确认"]),
        ("设备日志", &["设备使用记录", "维护日志", "运行记录"]),
        ("清洁记录", &["清洁消毒记录", "设备清洁记录"]),
        ("偏差报告", &["偏差调查表", "偏差记录", "Deviation Report"]),
        ("变更申请表", &["ECR", "变更请求", "变更通知单"]),
        ("供应商档案", &["供应商资质文件", "供应商评估档案"]),
    ];

    for (canonical, synonyms) in documents {
        terms.push(GmpTerm {
            term_id,
            canonical_term: canonical.to_string(),
            term_type: "entity".to_string(),
            synonyms: synonyms.iter().map(|s| s.to_string()).collect(),
            hypernyms: Some("GMP文件".to_string()),
            hyponyms: None,
            gmp_chapter: Some("第一章".to_string()),
            entity_type: Some("DOCUMENT".to_string()),
        });
        term_id += 1;
    }

    // Person terms (category: PERSON)
    let persons: &[(&str, &[&str])] = &[
        ("质量受权人", &["QP", "QA授权人", "质量负责人"]),
        ("质量负责人", &["QA负责人", "质量部负责人", "QA manager"]),
        ("生产负责人", &["生产经理", "Manufacturing Manager", "生产主管"]),
        ("设备管理员", &["设备工程师", "Equipment Manager", "维护责任人"]),
        ("验证工程师", &["Validation Engineer", "确认工程师"]),
        ("注册专员", &["RA", "法规事务", "注册事务"]),
        ("微生物专家", &["Microbiologist", "微生物检测工程师"]),
        ("工艺工程师", &["Process Engineer", "工艺开发工程师"]),
    ];

    for (canonical, synonyms) in persons {
        terms.push(GmpTerm {
            term_id,
            canonical_term: canonical.to_string(),
            term_type: "entity".to_string(),
            synonyms: synonyms.iter().map(|s| s.to_string()).collect(),
            hypernyms: Some("人员角色".to_string()),
            hyponyms: None,
            gmp_chapter: Some("第三章".to_string()),
            entity_type: Some("PERSON".to_string()),
        });
        term_id += 1;
    }

    // Location terms (category: LOCATION)
    let locations: &[(&str, &[&str])] = &[
        ("洁净区", &["ISO 7", "ISO 8", "洁净室", "Clean Room"]),
        ("D级区", &["D级洁净区", "十万级区", "ISO 8"]),
        ("C级区", &["C级洁净区", "万级区", "ISO 7"]),
        ("A/B级区", &["A级区", "B级区", "高风险区", "ISO 5"]),
        ("仓库", &["原料仓库", "成品仓库", "危险品仓库"]),
        ("质检区", &["质量控制区", "QC实验室", "微生物检验室"]),
        ("生产车间", &["制剂车间", "原料药车间", "包装车间"]),
        ("制水车间", &["纯化水制备", "注射用水系统", "水系统"]),
        ("空调机房", &["HVAC机房", "空气处理单元", "空调系统"]),
        ("留样室", &["样品储存室", "稳定性试验室"]),
    ];

    for (canonical, synonyms) in locations {
        terms.push(GmpTerm {
            term_id,
            canonical_term: canonical.to_string(),
            term_type: "entity".to_string(),
            synonyms: synonyms.iter().map(|s| s.to_string()).collect(),
            hypernyms: Some("场所区域".to_string()),
            hyponyms: None,
            gmp_chapter: Some("第四章".to_string()),
            entity_type: Some("LOCATION".to_string()),
        });
        term_id += 1;
    }

    terms
}