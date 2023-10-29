# A Dead Good Html To Rsx Converter

> Archived as I found the fantastic [rsx-rosetta](https://crates.io/crates/rsx-rosetta) project and use that to power the [dead-good-html-to-rsx-converter-web.shuttleapp.rs](https://dead-good-html-to-rsx-converter-web.shuttleapp.rs/)  instead.

I hate manually converting html to rsx. I hate it so much that I wrote a program to do it for me. It's not perfect, but it's better than doing it by hand.

## Usage

```rust
use dead_good_html_to_rsx_converter::convert;

fn main() {
    let html = r#"
        <div id="hero" class="container">
            <p>This is awesome!</p>
            <br />
        </div>
    "#;

    let rsx = convert(html);

    println!("{}", rsx.expect("Failed to convert html to rsx"));
}
```