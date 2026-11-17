use crate::content::Content;
use crate::highlight::escape;

pub fn rss(content: &Content) -> String {
    let base = content.config.base_url.trim_end_matches('/');
    let mut items = String::new();

    for post in &content.site.posts {
        let link = format!("{base}/posts/{}/", post.slug);
        let date = content
            .posts
            .iter()
            .find(|p| p.name == post.slug)
            .and_then(|p| p.date)
            .map(|d| d.rfc2822())
            .unwrap_or_default();

        items.push_str(&format!(
            "<item><title>{}</title><link>{link}</link><guid>{link}</guid><pubDate>{date}</pubDate><description>{}</description></item>\n",
            escape(&post.title),
            escape(&post.summary),
        ));
    }

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rss version=\"2.0\"><channel>\n<title>{}</title><link>{base}/</link><description>{}</description><language>en</language>\n{items}</channel></rss>\n",
        escape(&content.config.title),
        escape(&content.config.description),
    )
}

pub fn sitemap(content: &Content) -> String {
    let base = content.config.base_url.trim_end_matches('/');
    let mut paths = vec![
        String::from("/"),
        String::from("/posts/"),
        String::from("/play/"),
    ];
    paths.extend(content.pages.iter().map(|p| format!("/{}/", p.name)));
    paths.extend(content.posts.iter().map(|p| format!("/posts/{}/", p.name)));

    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );

    for path in paths {
        out.push_str(&format!("<url><loc>{base}{path}</loc></url>\n"));
    }

    out.push_str("</urlset>\n");
    out
}
