# 发布检查清单

## 预发布检查
- [ ] 所有测试通过: `cargo test -- --test-threads=1`
- [ ] clippy 无警告: `cargo clippy -- -D warnings`
- [ ] 代码已格式化: `cargo fmt --check`
- [ ] 性能基准测试运行正常: `cargo bench`
- [ ] 跨平台构建检查 (GitHub Actions)

## 版本更新
- [ ] 更新 `Cargo.toml` 版本号
- [ ] 更新 `README.md` 中的版本徽标
- [ ] 更新 `docs/usage.md` 中的版本引用
- [ ] 提交版本变更: `git commit -m "chore: bump version to x.y.z"`

## 发布构建
- [ ] 确保 CI 构建通过 (GitHub Actions)
- [ ] 创建并推送标签: `git tag vx.y.z && git push origin vx.y.z`
- [ ] 确认 Release 流水线完成
- [ ] 检查发布产物 (Windows .exe, Linux/macOS binary)

## 发布后
- [ ] 验证安装: `cargo install mf`
- [ ] 验证基本功能: `mf --version`
- [ ] 更新文档（如需要）
