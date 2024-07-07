use pulldown_cmark::{html, Parser};
use regex::Regex;
use serde::Deserialize;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Deserialize)]
struct FrontMatter {
  title: String,
  date: String,
  tags: Vec<String>,
}

fn main() {
  let input_dir = Path::new("data/articles");
  let output_dir = Path::new("src/routes/articles");
  for entry in WalkDir::new(input_dir).into_iter().filter_map(|e| e.ok()) {
    if entry.path().extension().map_or(false, |ext| ext == "md") {
      let input_path = entry.path();
      let relative_path = input_path.strip_prefix(input_dir).unwrap();
      let output_path = output_dir.join(relative_path).with_extension("svelte");

      let content = std::fs::read_to_string(input_path).unwrap();
      let (frontmatter, markdown) = extract_frontmatter(&content);
      let html_content = markdown_to_html(&markdown);
      let svelte_content = generate_svelte_component(&frontmatter, &html_content);

      std::fs::create_dir_all(output_path.parent().unwrap()).unwrap();
      std::fs::write(output_path, svelte_content).unwrap();
    }
  }
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
  format!(
          r#"<script lang="ts">
    import {{ onMount }} from 'svelte';

    let title = "{}";
    let date = "{}";
    let tags = {};

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
      {}
    </div>
  </article>

  <style>
    /* Add your styles here */
  </style>"#,
    frontmatter.title,
    frontmatter.date,
    serde_yaml::to_string(&frontmatter.tags).unwrap(),
    html_content
  )
}
