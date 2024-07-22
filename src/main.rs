use pulldown_cmark::{html, Parser};
use regex::Regex;
use serde::Deserialize;
use std::fs::File;
use std::io::{Write, Result};
use std::path::Path;
use walkdir::WalkDir;
use serde_json::json;
use chrono::NaiveDate;

#[derive(Deserialize)]
struct FrontMatter {
  #[serde(default)]
  slug: String,
  title: String,
  date: String,
  tags: Vec<String>,
}

fn main() {
  let input_dir = Path::new("data/articles");
  let output_dir = Path::new("src/routes/articles");
  let static_dir = Path::new("static/images/articles");

  let mut articles = Vec::new();
  for entry in WalkDir::new(input_dir).into_iter().filter_map(|e| e.ok()) {
    if entry.path().extension().map_or(false, |ext| ext == "md") {
      let input_path = entry.path();
      let relative_path = input_path.strip_prefix(input_dir).unwrap();
      let file_stem = relative_path.file_stem().unwrap().to_str().unwrap();
      let output_path = output_dir.join(file_stem).join("+page.svelte");

      let content = std::fs::read_to_string(input_path).unwrap();
      let (mut frontmatter, markdown) = extract_frontmatter(&content);
      frontmatter.slug = file_stem.to_string();
      let html_content = markdown_to_html(&markdown);
      let svelte_content = generate_svelte_component(&frontmatter, &html_content);

      std::fs::create_dir_all(output_path.parent().unwrap()).unwrap();
      std::fs::write(output_path, svelte_content).unwrap();

      articles.push(frontmatter);
    }
  }

  generate_article_data(&articles, output_dir).unwrap();

  let input_images = input_dir.join("images");
  if input_images.exists() {
    std::fs::create_dir_all(static_dir).unwrap();
    copy_dir_all(input_images, static_dir).unwrap();
  }
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
  std::fs::create_dir_all(&dst)?;
  for entry in std::fs::read_dir(src)? {
    let entry = entry?;
    let ty = entry.file_type()?;
    if ty.is_dir() {
      copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
    }
    else {
      std::fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
    }
  }
  Ok(())
}

fn extract_frontmatter(content: &str) -> (FrontMatter, String) {
  let re = Regex::new(r"(?s)^---\n(.*?)\n---\n(.*)$").unwrap();
  let captures = re.captures(content).unwrap();

  let frontmatter: FrontMatter = serde_yaml::from_str(&captures[1]).unwrap();
  let markdown = captures[2].to_string();

  (frontmatter, markdown)
}

fn markdown_to_html(markdown: &str) -> String {
  let parser = Parser::new(markdown);
  let mut html_output = String::new();
  html::push_html(&mut html_output, parser);
  let re = Regex::new(r#"<pre><code>([\s\S]*?)</code></pre>"#).unwrap();
  html_output = re.replace_all(&html_output, |caps: &regex::Captures| {
    let code = &caps[1];
    let language = if code.starts_with("python") {
      "language-python"
    }
    else {
      "language-none"
    };
    format!("<pre class=\"code-block\"><code class=\"{}\">{}</code></pre>", language, code)
  }).to_string();
  html_output
}

fn generate_article_data(articles: &Vec<FrontMatter>, output_dir: &Path) -> std::io::Result<()> {
  let output_path = output_dir.join("articleData.ts");
  let mut file = File::create(output_path)?;

  writeln!(file, "export const articles = [")?;
  for article in articles {
    writeln!(file, "  {{")?;
    writeln!(file, "    slug: '{}',", article.slug)?;
    writeln!(file, "    title: '{}',", article.title.replace("'", "\\'"))?;
    writeln!(file, "    date: '{}',", article.date)?;
    writeln!(file, "    tags: {:?}", article.tags)?;
    writeln!(file, "  }},")?;
  }
  writeln!(file, "];")?;

  Ok(())
}

fn generate_svelte_component(frontmatter: &FrontMatter, html_content: &str) -> String {
  let tags_json = serde_json::to_string(&frontmatter.tags).unwrap();
  let date = NaiveDate::parse_from_str(&frontmatter.date, "%Y-%m-%d").unwrap();
  let formatted_date = date.format("%B %d, %Y").to_string();

  let content_json = json!(html_content.replace("src=\"images/", "src=\"/images/articles/"));
  let profile_image = include_str!("static/profile_image.svg");

  format!(
    r#"<script>
    import {{ onMount }} from 'svelte';
    import Prism from 'prismjs';
    import 'prismjs/themes/prism-okaidia.css';
    import 'prismjs/components/prism-python';

    export const title = '{}';
    export const date = '{}';
    export const tags = {};

    let content = {};

    onMount(() => {{
      Prism.highlightAll();
    }});
  </script>

  <div class="title">
    <h1 class="title">{{title}}</h1>

    <div class="meta">
      <div class="profile" itemprop="author" itemscope="" itemtype="http://schema.org/Person" style="height:48px">
        <img itemprop="image" src='data:image/png;base64,{}'>
        <span class="mono authors">
          <a itemprop="name" href="https://shawnhagler.org">Shawn Hagler</a>
          <p class="subtitle">{{date}}</p>
        </span>
      </div>
    </div>
    <hr>

    <div class="content">
      {{@html content}}
    </div>
  </div>

  <style>
    * {{
      margin: 0;
      padding: 0;
      box-sizing: border-box;
      color: inherit;
      text-decoration: inherit;
    }}

    html {{
      background: var(--bg-0);
      color: var(--text-0);
      width: 100%;
      text-rendering: optimizeLegibility;
      font-feature-settings: "kern" 1;
      font-feature-settings: "liga" 1;
      min-width: 100vw;
      overflow-x: hidden;
      -webkit-text-size-adjust: 100%;
    }}

    @media all and (min-width:640px) {{
      html {{
        font-size: 16.5px;
      }}
    }}

    @media all and (min-width:720px) {{
      html {{
        font-size: 17px;
      }}
    }}

    @media all and (min-width:960px) {{
      html {{
        font-size: 18px;
      }}
    }}

    body {{
      background-color: #fffdf0;
      max-width: 944px;
      margin: 0 auto;
      padding: 0 24px;
      font-family: 'Berkely Mono', monospace;
    }}

    header, h1, h2, h3, .sans {{
      font-family: 'Berkely Mono', monospace;
      font-size: 18px;
    }}

    code, .mono, summary {{
      font-family: 'Berkely Mono', monospace;
      font-weight: 500;
    }}

    .img-right {{
      float: right;
      height: 300px;
      padding-left: 2em;
    }}

    body > header {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin: 2em 0;
    }}

    nav a {{
      margin-left: 1.5em;
      letter-spacing: 0.07em;
      font-size: .9rem;
    }}

    .m {{
      margin-left: 11%;
      position: relative;
    }}

    .r {{
      text-align: end;
    }}

    h1 {{
      font-size: 6em;
    }}

    .red {{
      color: #EF5350;
    }}

    article {{
      margin: 0 0 1rem -24px;
      padding-left: 20px;
      position: relative;
      border-left: solid 4px;
    }}

    article > a {{
      letter-spacing: 0.05em;
    }}

    article > div {{
      font-size: .9rem;
    }}

    article > time {{
      color: var(--text-1);
      font-size: .9rem;
      display: block;
      margin-bottom: 4px;
    }}

    article > div {{
      color: var(--text-1);
    }}

    article a {{
      color: var(--text-0);
      position: relative;
    }}

    article h1 {{
      font-size: 2rem;
      margin-bottom: 12px;
    }}

    @media screen and (min-width: 1248px) {{
      time {{
        position: absolute;
        left: 0;
        top: 0;
        transform: translateX(calc(-100% - 24px));
      }}
    }}

    @media screen and (max-width: 1248px) {{
      .shapes {{
        display: none;
      }}
    }}

    @media screen and (max-width: 1200px) {{
      .m {{
        margin-left: 0;
      }}
      .r {{
        text-align: left;
      }}

      hgroup {{
        margin-left: 0;
        margin-right: 0;
      }}

      h1 {{
        font-size: 4em;
        line-height: 100%;
      }}

      h2 {{
        font-size: 2em;
        line-height: 100%;
      }}
    }}

    body {{
      font-family: 'Berkely Mono', monospace;
    }}

    code, pre {{
      font-family: 'Berkely Mono', monospace;
    }}

    .header {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-top: 2rem;
      margin-bottom: 2rem;
    }}

    .header__logo {{
      font-family: sans-serif;
      font-size: 1.125rem;
    }}

    .header__nav-link {{
      margin-left: 1.5rem;
      font-size: 0.875rem;
      letter-spacing: 0.05em;
      font-family: sans-serif;
    }}

    .main {{
      max-width: 56rem;
      margin-left: auto;
      margin-right: auto;
      padding-left: 1.5rem;
      padding-right: 1.5rem;
    }}

    h1 {{
      font-size: 2rem;
      margin-bottom: 12px;
    }}

    .profile > img {{
      display: inline;
      object-fit: cover;
      height: 48px;
      width: 48px;
      border-radius: 100%;
      margin-right: 8px;
      background: var(--bg-1);
    }}

    img:not(.profile img) {{
      margin-top: 12px;
      margin-bottom: 12px;
    }}

    pre {{
      margin-top: 12px;
      margin-bottom: 12px;
    }}

    .authors {{
      position: absolute;
      margin-top: 4px;
      color: var(--text-1);
      font-size: 16px;
    }}

    .subtitle {{
      color: rgba(0, 0, 0, 66%);
      font-size: 16px;
    }}

    hr {{
      width: 164px;
      border: 2.5px solid;
      margin-top: 12px;
      margin-bottom: 32px;
    }}

    h3, h2 {{
      line-height: 24px;
    }}

    h2 {{
      font-size: 1.2em;
    }}

    h1, h2, h3 {{
      position: relative;
      margin: 1.2rem 0 0 2rem 0;
      margin-bottom: 12px;
      margin-top: 12px;
    }}

    h2:before {{
      content: '\#';
      position: absolute;
      margin-left: -19px;
    }}

    table {{
      border-collapse: separate;
      border-spacing: 10px;
    }}

    th, td {{
      padding: 10px;
      margin-bottom: 12px;
    }}
  </style>
  "#,
    frontmatter.title,
    formatted_date,
    tags_json,
    content_json,
    profile_image
  )
}
