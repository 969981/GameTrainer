# 测试矩阵与证据要求

## Mock / UI（M0）
- [ ] Gen3、Gen4、Gen5、Gen6、PC 五套 Manifest 注册成功。
- [ ] 加入第六个 Mock 不改 AppShell/全局导航。
- [ ] 跨游戏切换不复用前游戏 Session、Seed、Profile、预览令牌。
- [ ] unknown / disconnected / stale / conflict / restore-failed 有可理解 UI。
- [ ] 主题/搜索/键盘/1100×700、1366×768、1500×930 截图回归。

## Runtime（M1）
- [ ] 进程不存在、权限不足、架构不匹配、进程退出、PID 重用。
- [ ] ROM/EXE build、地区、revision 不兼容、Profile 改变。
- [ ] Guest Address 与 Host Address 类型分离，测试错误映射不会写入。

## Patch（M2）
- [ ] 预览 token 有时效、一次性；会话 epoch 变化拒绝。
- [ ] 改写前原字节冲突拒绝；同时插件重叠拒绝。
- [ ] 恢复只触达有所有权且可验证的区域；恢复失败不伪装成功。
- [ ] SQLite 只存字段白名单与错误摘要；无需持久化敏感 dump。

## 实机证据模板
- OS、架构、Runtime/模拟器版本、游戏版本/哈希（合法分享范围内）。
- 测试条件、步骤、预期、观察、日志文件位置、结果和已知限制。
- Mock/静态测试/集成测试/Windows 实机需要分别标注，不能互相替代。
