use pulldown_cmark::{html, Parser};
use regex::Regex;
use serde::Deserialize;
use std::path::Path;
use walkdir::WalkDir;
use serde_json::json;

#[derive(Deserialize)]
struct FrontMatter {
  title: String,
  date: String,
  tags: Vec<String>,
}

fn main() {
  let input_dir = Path::new("data/articles");
  let output_dir = Path::new("src/routes/articles");
  let static_dir = Path::new("static/images/articles");
  for entry in WalkDir::new(input_dir).into_iter().filter_map(|e| e.ok()) {
    if entry.path().extension().map_or(false, |ext| ext == "md") {
      let input_path = entry.path();
      let relative_path = input_path.strip_prefix(input_dir).unwrap();
      let file_stem = relative_path.file_stem().unwrap().to_str().unwrap();
      let output_path = output_dir.join(file_stem).join("+page.svelte");

      let content = std::fs::read_to_string(input_path).unwrap();
      let (frontmatter, markdown) = extract_frontmatter(&content);
      let html_content = markdown_to_html(&markdown);
      let svelte_content = generate_svelte_component(&frontmatter, &html_content);

      std::fs::create_dir_all(output_path.parent().unwrap()).unwrap();
      std::fs::write(output_path, svelte_content).unwrap();
    }
  }

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
  html_output
}

fn generate_svelte_component(frontmatter: &FrontMatter, html_content: &str) -> String {
  let tags_json = serde_json::to_string(&frontmatter.tags).unwrap();
  let content_json = json!(html_content.replace("src=\"images/", "src=\"/images/articles/"));

  format!(
          r#"<script context="module">
    export const title = "{}";
    export const date = "{}";
    export const tags = {};
  </script>

  <script lang="ts">
    import {{ onMount }} from 'svelte';

    let content = {};

    onMount(() => {{
      // Add any client-side logic here
    }});
  </script>

  <article>
    <h1>{{title}}</h1>
    <time datetime="{{date}}">{{date}}</time>
    <div class="tags">
      {{#each tags as tag}}
        <span class="tag">{{tag}}</span>
      {{/each}}
    </div>
    <div class="content">
      {{@html content}}
    </div>
  </article>

  <style>
    /* Add your styles here */
  </style>"#,
    frontmatter.title,
    frontmatter.date,
    tags_json,
    content_json
  )
}
