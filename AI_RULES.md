# Aurora Programming Language - AI Development Rules & Guidelines

> Bu dosya, Aurora dilinin geliştirilmesinde yer alan yapay zeka asistanları ve geliştiriciler için temel mimari kuralları, kodlama standartlarını ve çalışma disiplinini belirler.

---

## 1. Temel İlkeler ve Felsefe

1. **Performans ve Sıfır Maliyet (Zero-Cost Abstractions)**:
   - Yazılan her Rust modülü, gereksiz heap tahsislerinden (`clone()`, aşırı `Box` kullanımı) kaçınmalı, referanslar ve `&str` / `Cow` / `Span` yapılarını tercih etmelidir.
   - Algoritmik karmaşıklık her zaman göz önünde bulundurulmalı; Lexer ve Parser akışında O(N) tek geçişli veya minimum geri izlemeli (backtracking) yaklaşımlar seçilmelidir.

2. **Kullanıcı Dostu Hata Raporlama (Human-Centric Diagnostics)**:
   - Derleyici hataları Rust ve Elm düzeyinde açıklayıcı olmalıdır. Sadece hata mesajı değil, hatanın kaynak kodundaki satır/sütun koordinatları, hatayı gösteren kod parçacığı ve olası çözüm önerileri (hints) sunulmalıdır.

3. **Modüler ve Ayrıştırılmış Mimari (Separation of Concerns)**:
   - Kod tabanı kesinlikle bağımsız `crate`'lere bölünmelidir:
     - `aurora-lexer`: Sadece token üretir, söz dizimi semantiği bilmez.
     - `aurora-parser`: Sadece AST üretir, tip kontrolü yapmaz.
     - `aurora-ast`: AST düğümleri ve span tanımlarını içerir.
     - `aurora-sema`: Semantik analiz, tip çıkarımı ve sembol tablosunu yönetir.
     - `aurora-ir`: Sanal makine veya kod üreticisi için ara kod gösterimidir.
     - `aurora-vm`: Bayt kodunu en yüksek hızda çalıştırır.
     - `aurora-cli`: Kullanıcı arayüzü ve komut satırı deneyimini sağlar.

---

## 2. Rust Kodlama Standartları

1. **Idiomatic Rust**:
   - `unwrap()` ve `expect()` kullanımı kütüphane kodlarında **yasaktır**. Hatalar her zaman `Result<T, AuroraError>` veya ilgili crate'in hata tipiyle (`thiserror` / `miette`) yukarı fırlatılmalıdır. Sadece testlerde veya mantıksal olarak imkansız durumlarda açıklayıcı bir `expect("neden patlamayacağının açıklaması")` ile kullanılabilir.
   - `rustfmt` ve `clippy` standartlarına harfiyen uyulmalıdır. Her faz sonunda `cargo clippy --workspace --all-targets -- -D warnings` ve `cargo test --workspace` temiz geçmelidir.

2. **Hafıza Güvenliği**:
   - Aurora derleyicisinin kendi Rust kodu içinde `unsafe` blokları yalnızca kritik performans noktalarında (örneğin VM iç döngüsünde) ve detaylı `// SAFETY:` açıklamaları ile kullanılabilir.

3. **Test Odaklı Geliştirme (TDD / Regression Testing)**:
   - Eklenen her yeni dil özelliği için:
     - Birim testleri (`unit tests`)
     - Entegrasyon testleri (`tests/`)
     - Hatalı kodların düzgün yakalandığını doğrulayan "negative/diagnostic" testleri yazılmalıdır.

---

## 3. Faz Yönetimi ve Git Disiplini

1. **Faz Bazlı İlerleme**:
   - Geliştirme, `ROADMAP.md` üzerinde tanımlanan faz sırasına göre adım adım yürütülür.
   - Bir faz tamamlanmadan ve tüm testleri başarıyla geçmeden bir sonraki faza geçilmez.
   - **ÖNEMLİ KURAL**: Her faz tamamlandıktan sonra yapılan işler özetlenir, Git'e commit edilip push edilir ve **kullanıcıdan yeni komut beklenir**. Kullanıcı onay vermeden sonraki faza başlanmaz.

2. **Commit ve Push Konvansiyonu (Conventional Commits)**:
   - Format: `<type>(<scope>): <subject>`
   - Tipler:
     - `feat`: Yeni bir dil özelliği, parser kuralı, CLI komutu.
     - `fix`: Hata düzeltmesi, lexer/parser/vm bugfix.
     - `refactor`: Davranış değiştirmeyen kod iyileştirmesi.
     - `perf`: Performans optimizasyonu.
     - `test`: Test ekleme veya güncelleme.
     - `docs`: Dokümantasyon veya roadmap güncellemeleri.
     - `chore`: Bağımlılık, CI, konfigürasyon değişiklikleri.
   - Örnek: `feat(parser): implement Pratt parsing for binary arithmetic expressions`

3. **Dokümantasyon Bütünlüğü**:
   - Kod içi genel (`pub`) fonksiyonlar ve veri yapıları Rustdoc (`///`) ile belgelenmelidir.
   - Dilin kullanıcılarına yönelik söz dizimi ve standart kütüphane dokümantasyonu güncel tutulmalıdır.

