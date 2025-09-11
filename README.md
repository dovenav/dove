# dove

一个用 Rust 生成静态网页的简洁导航站点生成器，输出可直接部署到 Cloudflare Pages。支持主题目录与模板引擎（Tera），可自定义 HTML/CSS/JS；支持内/外网两套页面；内置搜索引擎切换与两种布局（default/ntp）。

## 快速开始

- 生成示例配置与默认主题：

  `cargo run -- init`

- 根据配置生成静态站点到 `dist/`：

 `cargo run -- build`

- 启动本地预览（默认监听 `127.0.0.1:8787`，启动前会先构建一次）：

  `cargo run -- preview --build-first`

生成完成后，打开 `dist/index.html`（外网版）或 `dist/intranet.html`（内网版）预览即可；若配置了 `base_path`，则文件位于 `dist/<base_path>/` 下。页头可相互切换。

如需“本地仅保留模板、配置放在 Gist”：不放置 `dove.yaml`，用 `--input-url` 或环境变量 `DOVE_INPUT_URL`/`DOVE_GIST_ID` 指定远程配置。
注意：远程加载功能位于可选特性 `remote` 中，默认未启用；请使用 `--features remote`。

### 构建时下载远程图标（并发）

当以 `--features remote` 构建并执行 `build/preview` 时，程序会尝试在构建阶段并发下载配置中的远程图标（`http/https` 或 `//` 开头），保存为本地文件并在最终页面中优先引用本地文件；若下载失败，则继续使用原远程链接。

- 存放目录（相对站点根）：`DOVE_ICON_DIR`，默认 `assets/icons`
- 下载并发数：`DOVE_ICON_THREADS`，默认 `8`

说明：`data:` 内联图标与本地相对路径图标不会被下载。

## 配置说明（dove.yaml）

示例：

```
site:
  title: 我的导航站
  description: 常用站点与工具集合
  color_scheme: auto   # auto | light | dark
  theme_dir: themes/default
  base_path: secretPath   # 可选：将站点生成到 dist/secretPath/
  # 可选：站点基础 URL（用于 canonical、sitemap、OG 等绝对地址）
  base_url: https://nav.example.com
  # 可选：OG 分享图片；相对路径将在结合 base_url 后生成绝对地址
  og_image: assets/favicon.svg
  # 可选：首页布局（default | ntp）
  layout: ntp
  # 站点地图默认设置（可选）
  sitemap:
    default_changefreq: weekly   # always|hourly|daily|weekly|monthly|yearly|never
    default_priority: 0.5        # 0.0 - 1.0
    # lastmod: 2025-09-09        # 不设置则用构建时间
  # 搜索引擎（可选）
  search_engines:
    - name: Google
      template: https://www.google.com/search?q={q}
    - name: Bing
      template: https://www.bing.com/search?q={q}
  default_engine: Google
  redirect:               # 跳转页设置（仅外网链接生成中间页）
    delay_seconds: 3      # 自动跳转倒计时（秒），0 表示不自动跳转
    default_risk: low     # 默认风险等级：low|medium|high
    utm:                  # 站点级 UTM（可选；字段留空则不追加）
      source: nav
      medium: referral
      campaign: homepage

groups:
  - category: 常用
    name: 搜索
    links:
      - name: Google
        url: https://www.google.com
        intro: 搜索引擎        # 兼容旧字段名 desc
        # details: "<p>可选：富文本 HTML 详情</p>"
        icon: assets/favicon.svg
        intranet: http://google.corp   # 可选：内网地址
        risk: medium                   # 可选：覆盖默认风险等级
        utm:                           # 可选：覆盖站点级 UTM
          source: nav
          medium: card
          campaign: homepage
  - category: 开发
    name: 文档/学习
    links:
      - { name: MDN Web Docs, url: https://developer.mozilla.org, intro: Web 开发文档 }
```

- `site.color_scheme` 控制颜色模式（自动/明亮/深色），也可在网页右上角按钮切换并记忆。
- `site.theme_dir` 指向主题目录。主题目录需包含 `templates/` 与 `assets/`。
- `site.base_path` 可选：将站点输出到 `dist/<base_path>/`，部署后访问路径形如 `https://domain/<base_path>/index.html`。
- `site.base_url`、`site.og_image` 可选：用于 SEO/canonical/OG。未设置 `base_url` 时，sitemap 使用相对地址且 `og_image` 仅在为绝对地址时输出。
- `site.layout` 可选：`default|ntp`，控制首页布局。
- `icon` 可为相对路径或外链 URL。若不设置，也可不显示图标。
- `links[].intranet` 可选，配置后会在“内网版页面”使用该地址；未配置时会回退到外网地址。
- `links[].intro` 简介；兼容旧字段名 `desc`。`links[].details` 为可选富文本 HTML，仅在详情页展示；未设置时回退显示简介文本。
- `links[].slug` 可选：显式指定外网中间页路径 `go/<slug>/` 的目录名；若未指定，则：
  - 默认用 `name` 生成 slug；
  - 当同名重复时，重复项将改用 `name+host` 组合生成 slug；
  - 若仍冲突，会在末尾追加 `-2`、`-3` 等序号确保唯一。
- `site.redirect` 跳转页设置（仅外网模式生成）：
  - `delay_seconds` 自动跳转倒计时；为 0 或缺省时不自动跳转。
  - `default_risk` 默认风险等级（low|medium|high）。
  - `utm` 站点级 UTM 参数（source/medium/campaign/term/content）。
- `site.sitemap` 站点地图默认设置：
  - `default_changefreq` 默认变更频率：`always|hourly|daily|weekly|monthly|yearly|never`
  - `default_priority` 默认优先级：`0.0 - 1.0`
  - `lastmod` 站点级最近更新时间（不设置则用构建时间）
- `links[].lastmod`、`links[].changefreq`、`links[].priority`：覆盖单个链接的站点地图字段。
- `links[].risk` 可选：覆盖默认风险等级。
- `links[].utm` 可选：覆盖站点级 UTM 参数。
- `site.search_engines` 与 `site.default_engine`：配置搜索引擎列表及默认项；模板可使用 `search_engines` 与 `engine_default` 变量。

## 主题结构

- `templates/index.html.tera`：首页模板（Tera）。可访问变量（常用）：
  - `site_title`、`site_desc`、`color_scheme`（`auto|light|dark`）、`layout`
  - `mode`（`external|intranet`）、`mode_other_label`（`外网|内网`）、`network_switch_href`、`has_intranet`
  - `categories`：分类列表（侧边栏）
  - `groups`：分组数组；每个分组包含 `name`、`category` 与 `links`
  - `links`：每个链接包含 `name`、`href`、`desc`、`icon`、`host`
  - `search_engines`、`engine_default`：搜索引擎选项与默认项
  - `meta_robots`：内网页会注入 `noindex,nofollow`
  - `canonical_url`、`og_image`：仅外网页面可用
- `templates/detail.html.tera`：链接详情/跳转提示页（仅外网生成）。可访问变量：
  - `site_title`、`site_desc`、`color_scheme`
  - `link_name`、`link_intro`、`link_details_html`、`link_icon`、`link_host`、`link_url`
  - `risk_class`（low|medium|high）、`risk_label`（低/中/高风险）
  - `has_delay`（bool）、`delay_seconds`（数字）
- `assets/`：静态资源（CSS/JS/图标等），会复制到输出目录的 `assets/`。

### 输出说明

- `index.html` 外网版导航（若设置 `base_path`，在 `dist/<base_path>/index.html`）
- `intranet.html` 内网版导航（同上；若 `--no-intranet` 则不生成且页面不显示切换按钮）
- `go/<slug>/index.html` 每个链接的详情/跳转提示页（仅外网版生成；导航页会将链接指向这些中间页）
- `sitemap.xml` 站点地图：包含 `index.html` 与所有外网详情页（带 `lastmod`、`changefreq`、`priority`）。
- `robots.txt` 基础抓取策略（默认 Allow: /）。

## 高级用法

```
cargo run --features remote -- build \
  --input-url https://gist.githubusercontent.com/<user>/<id>/raw/config.yaml \
  --out public --static-dir static --theme themes/default --base-path secretPath --no-intranet \
  --color-scheme dark --title "远程站点" --description "从远程加载"

# 预览更多参数
cargo run -- preview --addr 127.0.0.1:9090 --dir dist/secretPath
# 或基于远程配置推导目录（需启用 remote 特性）：
cargo run --features remote -- preview --build-first \
  --input-url https://gist.githubusercontent.com/<user>/<id>/raw/config.yaml \
  --color-scheme light --title "远程预览" --description "预览远程配置"
```

通过 Gist 加载配置（需启用 feature `remote`）：

```
# 使用 Gist（公开或私有）构建
cargo run --features remote -- build \
  --gist-id <GIST_ID> \
  --gist-file dove.yaml \  # 可选：不指定则取第一个文件
  --github-token <TOKEN> \  # 访问私有 Gist 时需要
  --auth-scheme Bearer \    # 授权方案，默认 token；也可用 Bearer
  --out public --static-dir static --theme themes/default --base-path secretPath --no-intranet \
  --color-scheme dark --title "Gist 站点" --description "从 Gist 加载"

# 使用 Gist 启动预览（启动前构建一次）
cargo run --features remote -- preview --build-first \
  --gist-id <GIST_ID> \
  --gist-file dove.yaml \  # 可选
  --github-token <TOKEN> \  # 私有 Gist 时必需
  --auth-scheme Bearer \    # 可选
  --color-scheme light --title "Gist 预览" --description "预览 Gist 配置"
```

说明：一旦指定了 `--input-url` 或 `--gist-id`，将忽略本地 `--input` 与自动发现的 `dove.yaml`，仅使用远程配置，并按 CLI/环境变量进行覆盖。

- `--input` 指定配置文件（默认自动寻找 `dove.yaml|dove.yml`）。
- `--out` 指定输出目录（默认 `dist/`）。
- `--static-dir` 指定额外静态资源目录，递归拷贝到输出目录，可覆盖主题资源。
- `--theme` 指定主题目录，优先级高于 `site.theme_dir`。
- `--base-path` 指定站点根路径（相对子路径），优先级高于 `site.base_path`。
- `--no-intranet` 仅生成外网版本页面（不生成 `intranet.html`，且页面不显示切换按钮）。
- 预览命令（preview）：
  - `--build-first` 启动前先构建一次。
  - `--addr` 监听地址（默认 `127.0.0.1:8787`）。
  - `--dir` 指定服务目录（若未指定，将根据配置推导 `dist/<base_path>`）。
  - `--open` 启动后自动在浏览器打开。

- 初始化命令（init）：
  - `--force` 强制覆盖已存在文件与主题。
  - `DIR` 可选目标目录（默认当前目录）。

### CLI 参数与环境变量

所有环境变量均有对应的 CLI 参数，便于在命令行直接覆盖：

- 常规：`--input`、`--input-url`、`--out`、`--static-dir`、`--theme`、`--base-path`、`--no-intranet`、（Preview）`--addr`
- 远程/Gist（需启用 `--features remote`）：`--gist-id`、`--gist-file`、`--github-token`、`--auth-scheme`
- 页面覆盖：`--color-scheme`（auto|light|dark）、`--title`、`--description`

优先级：CLI > 环境变量 > 配置文件 > 默认值。

来源选择：指定了 `--input-url` 或 `--gist-id` 时，将忽略本地 `--input` 与自动发现的 `dove.yaml`，仅读取远程配置并应用环境变量/CLI 覆盖。

环境变量清单（与上面 CLI 参数对应）：

- `DOVE_INPUT`：配置文件路径（等价于 `--input`）
- `DOVE_INPUT_URL`：配置文件 URL（等价于 `--input-url`，需启用 `remote` 特性）
- `DOVE_GIST_URL`：配置文件 URL 的别名，指向 gist 的 raw 链接
- `DOVE_GIST_ID`：配置所在 gist 的 ID（将通过 GitHub API 获取 raw_url）
- `DOVE_GIST_FILE`：配合 `DOVE_GIST_ID` 指定文件名（不指定则取第一个文件）
- `DOVE_GITHUB_TOKEN`：访问私有 Gist 时的 token（会作为 `Authorization: token <TOKEN>` 加到请求头）
- `DOVE_AUTH_SCHEME`：可选，授权方案（默认 `token`，也可设为 `Bearer` 或其他值，最终头格式为 `Authorization: <SCHEME> <TOKEN>`）
- `DOVE_OUT`：输出目录（等价于 `--out`）
- `DOVE_PREVIEW_ADDR`：预览监听地址（等价于 `--addr`）
- `DOVE_STATIC`：静态资源目录（等价于 `--static-dir`）
- `DOVE_THEME`：主题目录（等价于 `--theme`）
- `DOVE_THEME_DIR`：主题目录（`DOVE_THEME` 的别名）
- `DOVE_BASE_PATH`：站点根路径（等价于 `--base-path`）
- `DOVE_NO_INTRANET`：是否仅生成外网（真值如 `1/true/yes/on` 有效）
- `DOVE_COLOR_SCHEME`：页面配色方案覆盖（`auto|light|dark`）
- `DOVE_TITLE`：覆盖站点标题（仅影响渲染，不修改配置文件）
- `DOVE_DESCRIPTION`：覆盖站点描述（仅影响渲染，不修改配置文件）

优先级：CLI > 环境变量 > 配置文件 > 默认值。

私有 Gist 建议：使用 `DOVE_INPUT_URL` 指向 Gist 的 raw 链接，或设置 `DOVE_GIST_ID` 并提供 `DOVE_GITHUB_TOKEN`（可配合 `DOVE_AUTH_SCHEME=Bearer`）；两种方式都会在请求中自动携带 `Authorization` 头。以上需启用 `remote` 特性。

### 版本信息（页脚）

本主题页脚默认显示构建版本：`… · v{{ build_version }}`。版本号在构建时注入，支持三种来源（按优先级）：

- CLI 参数：`--build-version <VER>`（最高优先级）
- 环境变量：`DOVE_BUILD_VERSION=<VER>`
- 回退：crate 版本（`CARGO_PKG_VERSION`）

常用用法：

```
# 本地构建（推荐其一）
cargo run -- build --build-version "$(git describe --tags --always --dirty --long)"
# 或
DOVE_BUILD_VERSION="$(git describe --tags --always --dirty --long)" cargo run -- build

# 预览时也可带上版本（自动重建将沿用该值）
cargo run -- preview --build-first --build-version "$(git describe --tags --always --dirty --long)"
```

GitHub Actions（示例）：工作流中先计算版本，再通过 CLI 传递：

```
- name: Compute version
  id: ver
  run: echo "version=$(git describe --tags --always --dirty --long)" >> $GITHUB_OUTPUT
- name: Build
  run: cargo run -- build --build-version "${{ steps.ver.outputs.version }}"
```

说明：Actions 示例也同时设置了环境变量 `DOVE_BUILD_VERSION`（可选），但最终以 `--build-version` 为准；两者保持一致即可。

### 分类与分组（一级/二级）

- 一级分类：使用 `groups[].category` 字段；用于侧边栏分类列表（如“常用/开发/学习”）。未填写时默认归入“全部”。
- 二级分类（分组）：使用 `groups[].name` 字段；它是内容区中每个区块的标题。相同 `category` 的多个分组会归属到同一个一级分类下展示。
- 渲染规则：
  - 仅当某分组内“有可展示链接”时，才会渲染该分组；
  - 仅当至少有一个分组归属于某一级分类且该分组有可展示链接时，才会在侧边栏显示该一级分类；
  - 外网页面只统计设置了 `url` 的链接；内网页面统计 `intranet` 或回退 `url` 的链接。
  - 搜索时，仅显示包含匹配结果的分组；侧边栏仅显示仍有结果的一级分类。
  - 当分类列表为空（无可展示分类）时，侧边栏不渲染（隐藏）。

示例：

```
groups:
  - category: 常用
    name: 搜索
    links:
      - { name: Google, url: https://www.google.com, intro: 搜索引擎 }
      - { name: Bing,   url: https://www.bing.com }

  - category: 常用
    name: 工具
    links:
      - { name: TinyPNG, url: https://tinypng.com }

  - category: 开发
    name: 文档/学习
    links:
      - { name: MDN, url: https://developer.mozilla.org }
```

说明：如果某个分组没有可展示的链接（例如外网模式下全部缺少 `url`），该分组与其所属分类不会出现在页面中。

## 部署到 GitHub Pages

你可以使用 GitHub Actions 将 dove 站点自动发布到 GitHub Pages。可直接参考示例仓库：[dove-private](https://github.com/dovenav/dove-private)（仅包含 `dove.yaml` 与工作流），按同样方式搭建你自己的“配置仓库”。

步骤摘要：

- 在你的配置仓库中添加工作流（`.github/workflows/deploy.yml`）：
  - Checkout 配置仓库；
  - Checkout 本项目 [dovenav/dove](https://github.com/dovenav/dove) 到子目录（例如 `dove/`）；
  - 安装 Rust 工具链（如 `dtolnay/rust-toolchain@stable`）；
  - 将 `dove.yaml` 拷贝到 `dove/` 并执行 `cargo run -- build`；
  - 使用 `actions/configure-pages`、`actions/upload-pages-artifact`、`actions/deploy-pages` 发布 `dove/dist`；
- 在仓库 Settings → Pages 中将 Source 设置为 “GitHub Actions”。
- 若为 Project Pages（`https://<user>.github.io/<repo>/`），建议在 `dove.yaml` 设置 `site.base_path: <repo>`；User/Org Pages 通常无需设置。

完整可用的工作流与详细说明见 [dove-private/README.md](https://github.com/dovenav/dove-private/blob/main/README.md)。

## 部署到 Cloudflare Workers（可选）

也可以将 `dove/dist` 作为静态资源部署到 Cloudflare Workers，参考示例与说明见：

- [dove-private/README.md 中的 Workers 部署章节](https://github.com/dovenav/dove-private/blob/main/README.md)
- 示例工作流：`dove-private/.github/workflows/deploy-worker.yml`
- 配置：`dove-private/wrangler.toml`

## 部署到 Cloudflare Pages

推荐两种方式：

1) 本地生成后上传
- 本地执行 `cargo run -- build` 生成 `dist/`。
- 在 Cloudflare Pages 新建项目，选择 “直接上传（Direct Upload）”，上传 `dist/` 目录内容。

2) 连接仓库并跳过构建
- 在 CI/CD 或本地生成 `dist/` 并提交到仓库。
- 在 Cloudflare Pages 连接该仓库，Framework 选择 None。
- Build command 留空或填 `echo skip`，Output directory 填 `dist`。

> 说明：Cloudflare Pages 默认并不会预装 Rust 工具链，不建议在 Pages 侧执行 `cargo` 构建。

## 许可

MIT

## 捐赠支持

如果这个项目对你有帮助，欢迎扫码打赏支持（感谢！）：

<div align="center">

<img src="themes/default/assets/dove.PNG" alt="微信捐赠二维码" width="260"/>
<img src="themes/default/assets/evm.JPG" alt="区块链EVM: Etherum, BSC, Polygon" width="260"/>
<img src="themes/default/assets/torn.png" alt="TRON Chain" width="260"/>

</div>
 
