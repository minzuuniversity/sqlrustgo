# V312-11-Fix: Double-Quoted Identifier Handling - Tasks

## Task 1: 分析Lexer结构
- [ ] 查看lexer.rs的`next_token`函数
- [ ] 确认没有处理`"`字符
- [ ] 编写测试验证问题

## Task 2: 实现read_quoted_identifier方法
- [ ] 添加`fn read_quoted_identifier(&mut self) -> String`
- [ ] 跳过开始的双引号
- [ ] 读取内容直到结束双引号
- [ ] 返回不带引号的内容

## Task 3: 添加双引号处理分支
- [ ] 在`match ch`中添加`'"'`分支
- [ ] 调用`Token::Identifier(self.read_quoted_identifier())`

## Task 4: 集成测试
- [ ] 运行case_insensitive_alter.test
- [ ] 验证测试通过
- [ ] 运行完整sqllogictest确认无回归

## Task 5: 门禁集成
- [ ] 更新sqllogictest_gate.sh
- [ ] 验证gate通过
