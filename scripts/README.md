## Scripts

本目录包含用于构建、测试和部署的实用脚本。

### update-readme-version.sh

更新 README 文件中版本号的脚本。

```bash
./scripts/update-readme-version.sh <new-version>
```

- 自动更新所有 README 中的版本号
- 保持文档版本一致性
- 依赖 `python3`（可通过 `PYTHON_BIN` 环境变量覆盖）
