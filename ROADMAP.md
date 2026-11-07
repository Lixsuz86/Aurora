# Aurora Programming Language - Master Roadmap

> **Aurora**: Yüksek seviyeli zarafet, sistem seviyesinde performans ve mutlak kontrol.
> *"High-level ergonomics, low-level power."*

---

## 1. Vizyon ve Felsefe

Aurora, modern yazılım geliştiricilerinin iki ayrı dünya arasında (hızlı prototipleme sağlayan yüksek seviyeli diller ile donanımı doğrudan kontrol eden sistem dilleri) seçim yapmak zorunda kalmasını ortadan kaldırmayı hedefler.

* **Söz Dizimi Zarafeti**: Python, Swift ve Rust'ın en temiz yanlarını birleştiren, minimum söz dizimsel gürültüye (noise) sahip, okunabilir ve hızlı yazılabilen modern syntax.
* **Tavizsiz Performans**: Sıfır maliyetli soyutlamalar (zero-cost abstractions), agresif optimizasyonlar ve C/Rust seviyesinde çalışma hızı.
* **Çift Modlu Bellek Yönetimi**:
  * **Güvenli Mod (Safe/High-Level)**: Otomatik yaşam döngüsü (RAII + Akıllı Referans/Region/Ownership analizi), çöp toplayıcı (GC) duraksamaları olmadan tam bellek güvenliği.
  * **Sistem Modu (Low-Level/Unsafe)**: Doğrudan işaretçi (raw pointer) manipülasyonu, özel bellek tahsis ediciler (custom allocators), satır içi (inline) assembly ve donanım kayıtlarına doğrudan erişim.
* **Modern Tip Sistemi**: Statik, güçlü ve tip çıkarımına (type inference) sahip. Null güvenliği (`T?` / `Option<T>`), cebirsel veri tipleri (enums/variants), pattern matching ve trait/interface tabanlı kompozisyon.

---

## 2. Geliştirme Yol Haritası (Phased Roadmap)

```
[Faz 1: Temeller & Altyapı] ──▶ [Faz 2: Lexer & Tokenizer] ──▶ [Faz 3: AST & Parser]
          │                                                               │
          ▼                                                               ▼
[Faz 6: CLI & Test Suite]  ◀── [Faz 5: Bytecode VM / JIT]  ◀── [Faz 4: Semantik Analiz & Tipler]
          │
          ▼
[Faz 7: Standart Kütüphane] ──▶ [Faz 8: Low-Level Bellek & FFI] ──▶ [Faz 9: LLVM / Native Backend]
```

### Faz 1: Çalışma Alanı, Mimari Tasarım ve Geliştirme Araçları
- [x] Rust Workspace mimarisinin kurulması (`aurora-lexer`, `aurora-parser`, `aurora-ast`, `aurora-sema`, `aurora-vm`, `aurora-cli`).
- [x] Linter, formatlayıcı ve test altyapısının konfigürasyonu (`rustfmt.toml`, Clippy kuralları).
- [x] Detaylı Roadmap ve AI kurallarının oluşturulması (`ROADMAP.md`, `AI_RULES.md`, `.cursorrules`).
- [x] Git sürüm kontrolünün başlatılması ve ilk commit/push yapısının hazırlanması.

### Faz 2: Lexer & Kaynak Yönetimi (Tokenization)
- [x] UTF-8 destekli, sıfır kopyalı (zero-copy) Lexer tasarımı.
- [x] Detaylı kaynak konumu takibi (`Span`, `SourceLocation`, `SourceMap`).
- [x] Anahtar kelimeler, tanımlayıcılar (identifiers), sayılar (hex, bin, octal, float, integer with type suffixes), karakter ve ham dizeler (raw strings), operatörler.
- [x] Renkli ve açıklayıcı söz dizimi hata raporlama motoru (diagnostics with labels & hints, source snippets).

### Faz 3: AST & Pratt Parser
- [x] Zengin Soyut Söz Dizimi Ağacı (AST) tanımları.
- [x] Pratt Parsing algoritması ile öncelik tabanlı (precedence) ikili/tekli operatör çözümleme.
- [x] İfadeler (Expressions): Değişken atamaları, fonksiyon çağrıları, lambda ifadeleri, match blokları, if-else, döngüler (`while`, `for`, `loop`), pipeline (`|>`), dizi indeksleme, üye erişimi, aralıklar.
- [x] Tanımlamalar (Declarations): `fn`, `let`, `mut`, `const`, `struct`, `enum`, `trait`, `impl`, `type`, `use`, `unsafe fn`.
- [x] Hata kurtarma (Error recovery): Parser'ın ilk hatada durmayıp sonraki hataları da tespit edebilmesi (`synchronize`).

### Faz 4: Semantik Analiz & Tip Sistemi (Sema)
- [x] Kapsam ve Sembol Tablosu (Scope & Symbol Table) hiyerarşisi.
- [x] Tip Çıkarımı (Local Type Inference - Hindley-Milner / Bidirectional type checking).
- [x] Statik Tip Denetimi: İlkel tipler (`i8..i128`, `u8..u128`, `f32`, `f64`, `bool`, `string`), bileşik tipler, jenerikler (Generics).
- [x] Null Güvenliği: `Option<T>` / `T?` ve unwrap kuralları.
- [x] Değişkenlik (Mutability) ve yaşam süresi/sahiplik kurallarının doğrulanması.

### Faz 5: Ara Kod (IR) ve Bytecode VM / JIT Motoru
- [ ] Optimize edilebilir Aurora IR (Yığın veya Kayıt tabanlı Bytecode).
- [ ] Hızlı Yürütme Sanal Makinesi (Aurora VM):
  - Komut kümesi (OpCodes): Aritmetik, mantıksal, bellek yükleme/saklama, atlama (jumps), çağrı yığını (call stack).
  - Değer temsili (Tagged Unions veya NaN-boxing ile minimum bellek ayak izi).
- [ ] Performans profilleme ve mikro benchmark altyapısı (Criterion).

### Faz 6: Aurora CLI & Geliştirici Deneyimi (DX)
- [ ] `aurora run <file.au>`: Hızlı çalıştırma modu.
- [ ] `aurora build <file.au>`: Derleme modu.
- [ ] `aurora repl`: İnteraktif kabuk (Read-Eval-Print Loop).
- [ ] `aurora check`: Hızlı söz dizimi ve tip denetimi (kod çalıştırmadan).
- [ ] `aurora fmt`: Kod biçimlendirici.
- [ ] Kapsamlı entegrasyon test takımı ve örnek Aurora programları (`examples/`).

### Faz 7: Standart Kütüphane (Aurora Core / Stdlib)
- [ ] Giriş/Çıkış (I/O, dosya işlemleri, konsol renklendirme).
- [ ] Temel veri yapıları: Dinamik Diziler (`Array<T>`), Hash Tabloları (`Map<K, V>`), Kümeler (`Set<T>`).
- [ ] Matematik ve dize (string) manipülasyon yardımcıları.
- [ ] Eşzamanlılık ve Kanallar (Channels, async/await temelleri).

### Faz 8: Düşük Seviyeli Sistem Yetenekleri & FFI
- [ ] `unsafe` blokları ve doğrudan bellek işaretçileri (`*const T`, `*mut T`).
- [ ] Manuel bellek yönetimi API'leri (`alloc`, `free`, `realloc`).
- [ ] C ABI Uyumluluğu: C kütüphanelerini doğrudan dinamik/statik olarak bağlama (Zero-overhead C FFI).
- [ ] Bit düzeyinde manipülasyon ve donanım kontrol yetenekleri.

### Faz 9: LLVM / Cranelift ile Native AOT Derleme
- [ ] Aurora IR -> LLVM IR çevirici.
- [ ] Platforma özgü optimize edilmiş makine kodu (ELF, PE/COFF, Mach-O) üretimi.
- [ ] LTO (Link-Time Optimization) ve agresif optimizasyon bayrakları.

---

## 3. Örnek Aurora Söz Dizimi Önizlemesi

```aurora
// Yüksek Seviyeli Zarafet (High-Level Elegance)
struct User {
    id: u64,
    name: string,
    email: string?,
}

fn calculate_scores(numbers: [i32]) -> i32 {
    return numbers
        |> filter(|x| x > 0)
        |> map(|x| x * 2)
        |> sum()
}

// Düşük Seviyeli Güç & Performans (Low-Level Control)
unsafe fn fast_memory_copy(dest: *mut u8, src: *const u8, count: usize) {
    let mut i: usize = 0
    while i < count {
        *dest.offset(i) = *src.offset(i)
        i += 1
    }
}

fn main() {
    let message = "Merhaba Aurora!"
    println(message)
    
    let scores = [10, -5, 20, 30]
    let total = calculate_scores(scores)
    println("Toplam: {total}")
}
```

