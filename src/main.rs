use pulldown_cmark::{html, Parser, Options};
use regex::Regex;
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::{Write, Result};
use std::path::Path;
use walkdir::WalkDir;
use serde_json::json;
use chrono::NaiveDate;

struct ContentType {
  input_dir: &'static str,
  output_dir: &'static str,
  static_dir: &'static str,
  is_article: bool,
}

#[derive(Serialize, Deserialize)]
struct Author {
  name: String,
  url: Option<String>,
}

#[derive(Deserialize)]
struct FrontMatter {
  #[serde(default)]
  slug: String,
  title: String,
  #[serde(default)]
  authors: Vec<Author>,
  date: String,
  tags: Vec<String>,
}

fn main() {
  let content_types = vec![
    ContentType {
      input_dir: "data/articles",
      output_dir: "src/routes/articles",
      static_dir: "static/images/articles",
      is_article: true,
    },
    ContentType {
      input_dir: "data/projects",
      output_dir: "src/routes/projects",
      static_dir: "static/images/projects",
      is_article: false,
    },
  ];

  for content_type in content_types {
    let frontmatters = process_content(&content_type);
    generate_data(&frontmatters, Path::new(content_type.output_dir), content_type.is_article)
      .unwrap_or_else(|e| eprintln!("Error generating data: {}", e));

    let input_images = Path::new(content_type.input_dir).join("images");
    if input_images.exists() {
      std::fs::create_dir_all(content_type.static_dir)
        .unwrap_or_else(|e| eprintln!("Error creating directory {}: {}", content_type.static_dir, e));
      copy_dir_all(&input_images, Path::new(content_type.static_dir))
        .unwrap_or_else(|e| eprintln!("Error copying images: {}", e));
    }
  }
}

fn process_content(content_type: &ContentType) -> Vec<FrontMatter> {
  WalkDir::new(content_type.input_dir)
    .into_iter()
    .filter_map(|entry| entry.ok())
    .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
    .map(|entry| {
      let input_path = entry.path();
      let relative_path = input_path.strip_prefix(content_type.input_dir).unwrap();
      let file_stem = relative_path.file_stem().unwrap().to_str().unwrap();
      let output_path = Path::new(content_type.output_dir).join(file_stem).join("+page.svelte");

      let content = std::fs::read_to_string(input_path)
        .unwrap_or_else(|e| panic!("Error reading file {}: {}", input_path.display(), e));
      let (mut frontmatter, markdown) = extract_frontmatter(&content);
      frontmatter.slug = file_stem.to_string();
      let html_content = markdown_to_html(&markdown);
      let svelte_content = generate_svelte_component(&frontmatter, &html_content, content_type.is_article);

      std::fs::create_dir_all(output_path.parent().unwrap())
        .unwrap_or_else(|e| panic!("Error creating directory for {}: {}", output_path.display(), e));
      std::fs::write(&output_path, svelte_content)
        .unwrap_or_else(|e| panic!("Error writing to {}: {}", output_path.display(), e));

      frontmatter
    })
    .collect()
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
  let mut options = Options::empty();
  options.insert(Options::ENABLE_TABLES);
  let parser = Parser::new_ext(markdown, options);
  let mut html_output = String::new();
  html::push_html(&mut html_output, parser);

  let block_math_regex = Regex::new(r"<p>\$\$([\s\S]*?)\$\$</p>").unwrap();
  html_output = block_math_regex.replace_all(&html_output, |caps: &regex::Captures| {
    format!("\\[{}\\]", &caps[1])
  }).to_string();

  let inline_math_regex = Regex::new(r"\$([^\$\n]+?)\$").unwrap();
  html_output = inline_math_regex.replace_all(&html_output, |caps: &regex::Captures| {
    format!("\\({}\\)", &caps[1])
  }).to_string();

  let list_regex = Regex::new(r"(<[ou]l>(?:\s*<li>.*?</li>\s*)+</[ou]l>)").unwrap();
  html_output = list_regex.replace_all(&html_output, |caps: &regex::Captures| {
    format!("<div style=\"margin-left: 2em;\">{}</div>", &caps[1])
  }).to_string();

  let re = Regex::new(r#"<pre><code>([\s\S]*?)</code></pre>"#).unwrap();
  html_output = re.replace_all(&html_output, |caps: &regex::Captures| {
    let code = &caps[1];
    let language = if code.starts_with("python") {
      "language-python"
    }
    else if code.starts_with("vhdl") {
      "language-vhdl"
    }
    else if code.starts_with("cpp") {
      "language-cpp"
    }
    else if code.starts_with("c") {
      "language-c"
    }
    else {
      "language-none"
    };
    format!("<pre class=\"code-block\"><code class=\"{}\">{}</code></pre>", language, code)
  }).to_string();
  html_output
}

fn generate_data(frontmatters: &Vec<FrontMatter>, output_dir: &Path, is_article: bool) -> std::io::Result<()> {
  let file_name = if is_article { "articleData.ts" } else { "projectData.ts" };
  let output_path = output_dir.join(file_name);
  let mut file = File::create(output_path)?;

  let var_name = if is_article { "articles" } else { "projects" };
  writeln!(file, "export const {} = [", var_name)?;
  for frontmatter in frontmatters {
    writeln!(file, "  {{")?;
    writeln!(file, "    slug: '{}',", frontmatter.slug)?;
    writeln!(file, "    title: '{}',", frontmatter.title.replace("'", "\\'"))?;
    writeln!(file, "    authors: [")?;
    for author in &frontmatter.authors {
      write!(file, "      {{ name: '{}', ", author.name.replace("'", "\\'"))?;
      if let Some(url) = &author.url {
        writeln!(file, "url: '{}' }},", url.replace("'", "\\'"))?;
      } else {
        writeln!(file, "url: null }},")?;
      }
    }
    writeln!(file, "    ],")?;
    writeln!(file, "    date: '{}',", frontmatter.date)?;
    writeln!(file, "    tags: {:?}", frontmatter.tags)?;
    writeln!(file, "  }},")?;
  }
  writeln!(file, "];")?;

  Ok(())
}

fn generate_svelte_component(frontmatter: &FrontMatter, html_content: &str, is_article: bool) -> String {
  let tags_json = serde_json::to_string(&frontmatter.tags).unwrap();
  let authors_json = serde_json::to_string(&frontmatter.authors).unwrap();
  let date = NaiveDate::parse_from_str(&frontmatter.date, "%Y-%m-%d").unwrap();
  let formatted_date = date.format("%B %d, %Y").to_string();

  let image_path = if is_article { "images/articles" } else { "images/projects" };
  let content_json = json!(html_content.replace(&format!("src=\"images/"), &format!("src=\"/{}/", image_path)));
  let profile_image = include_str!("static/profile_image.svg");

  format!(
    r#"<script>
    import {{ onMount }} from 'svelte';
    import Prism from 'prismjs';
    import 'prismjs/themes/prism-okaidia.css';
    import 'prismjs/components/prism-python';
    import 'prismjs/components/prism-vhdl';
    import 'prismjs/components/prism-c';
    import 'prismjs/components/prism-cpp';

    export const title = '{}';
    export const date = '{}';
    export const tags = {};
    export const authors = {};

    let content = {};

    onMount(() => {{
      Prism.highlightAll();

      window.MathJax = {{
        tex: {{
          inlineMath: [['\\(', '\\)']],
          displayMath: [['\\[', '\\]'], ['$$', '$$']],
          processEscapes: true,
          processEnvironments: true
        }},
        options: {{
          skipHtmlTags: ['script', 'noscript', 'style', 'textarea', 'pre']
        }}
      }};

      const script = document.createElement('script');
      script.src = 'https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-chtml.js';
      script.async = true;
      document.head.appendChild(script);

      script.onload = () => {{
        setTimeout(() => {{
          MathJax.typesetPromise().catch((err) => {{
            console.error('MathJax error:', err);
          }});
        }}, 100);
      }};
    }});
  </script>

  <div class="title">
    <h1 class="title">{{title}}</h1>

    <div class="meta">
      <div class="profile" itemprop="author" itemtype="http://schema.org/Person" style="height:48px">
        <!-- svelte-ignore a11y-img-redundant-alt -->
        <img itemprop="image" src='data:image/png;base64,{profile_image}'>
        <span class="mono authors">
          {{#each authors as author, index}}
            {{#if author.url}}
              <a itemprop="name" href="{{author.url}}">{{author.name}}</a>
            {{:else}}
              <span itemprop="name">{{author.name}}</span>
            {{/if}}
            {{#if index < authors.length - 1}}<span class="ampersand">&amp;</span>{{/if}}
          {{/each}}
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
    .authors .ampersand {{
      display: inline-block;
      padding-right: 0.5em;
    }}
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
      font-family: 'Berkeley Mono', monospace;
    }}

    header, h1, h2, h3, .sans {{
      font-family: 'Berkeley Mono', monospace;
      font-size: 18px;
    }}

    code, .mono, summary {{
      font-family: 'Berkeley Mono', monospace;
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
      font-weight: 700;
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
      font-family: 'Berkeley Mono', monospace;
    }}

    code, pre {{
      font-family: 'Berkeley Mono', monospace;
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

    :not(.hgroup) h2:before {{
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
    authors_json,
    content_json,
  )
}
