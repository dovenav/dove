# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- 新增 `--generate-intermediate-page` 命令行参数，用于控制是否生成中间页
- 新增 `DOVE_GENERATE_INTERMEDIATE_PAGE` 环境变量，用于控制是否生成中间页
- 新增离线功能支持，通过 Service Worker 实现网页离线可用
- 当 `--generate-intermediate-page=false` 时，链接将直接跳转到目标地址，不生成中间页

### Changed
- 优化了命令行参数处理逻辑
- 改进了文档说明

## [0.1.0] - 2025-09-09

### Added
- Initial release