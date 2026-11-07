# Aurora Programming Language

```
      /\                                
     /  \  _   _ _ __ ___  _ __ __ _ 
    / /\ \| | | | '__/ _ \| '__/ _` |
   / ____ \ |_| | | | (_) | | | (_| |
  /_/    \_\__,_|_|  \___/|_|  \__,_|
```

> **Aurora**: Yüksek seviyeli zarafet, sistem seviyesinde performans ve mutlak kontrol.
> *"High-level ergonomics, low-level power."*

---

## Genel Bakış

Aurora; modern, okunabilir ve ekspresif bir söz dizimini C/Rust düzeyinde sıfır maliyetli soyutlamalar (zero-cost abstractions) ve donanım kontrolüyle birleştiren yeni nesil bir sistem ve genel amaçlı programlama dilidir.

### Temel Özellikler
* **Zarif Söz Dizimi**: Python ve Swift benzeri temiz, okunabilir, minimum noktalama işareti gerektiren modern syntax.
* **Tavizsiz Performans**: Rust ile geliştirilmiş, gereksiz heap tahsislerinden arındırılmış yüksek performanslı derleyici ve sanal makine.
* **Çift Modlu Bellek Mimarisi**:
  * *High-level Safe*: GC duraksaması olmadan otomatik güvenli bellek (RAII & Statik Yaşam Süresi).
  * *Low-level System*: Doğrudan ham işaretçiler (`*const T`, `*mut T`), manuel tahsis ve C FFI.
* **Gelişmiş Tip Sistemi**: Güçlü statik tipleme, yerel tip çıkarımı, null güvenliği (`Option<T>`), pattern matching.

---

## Proje Mimarisi (Workspace Crates)

* `crates/aurora-ast`: Soyut Söz Dizimi Ağacı (AST) düğümleri ve kaynak kod konumlandırma (`Span`).
* `crates/aurora-lexer`: Sıfır kopyalı (zero-copy), UTF-8 destekli tokenizasyon motoru.
* `crates/aurora-parser`: Pratt algoritması tabanlı öncelik denetimli söz dizimi ayrıştırıcısı.
* `crates/aurora-sema`: Semantik analiz, tip çıkarımı ve hiyerarşik sembol tablosu.
* `crates/aurora-vm`: Komut kümesi (Bytecode OpCodes) ve yüksek performanslı sanal makine.
* `crates/aurora-cli`: `aurora` komut satırı arayüzü (`run`, `build`, `check`, `repl`).

---

## Kurulum ve Çalıştırma

Projeyi derlemek ve testleri çalıştırmak için Rust ortamı gereklidir.

```bash
# Projeyi derleme
cargo build --workspace

# Testleri çalıştırma
cargo test --workspace

# Clippy linter denetimi
cargo clippy --workspace

# CLI uygulamasını çalıştırma
cargo run -p aurora-cli -- --help
```

---

## Yol Haritası ve Kurallar

* Detaylı aşamalar ve fazlar için: [`ROADMAP.md`](ROADMAP.md)
* Geliştirme ilkeleri ve yapay zeka standartları için: [`AI_RULES.md`](AI_RULES.md)

