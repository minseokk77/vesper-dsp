"use client";

import type { CSSProperties } from "react";
import { useEffect, useState } from "react";
import { content, type Language } from "./content";

const RELEASES = {
  dsp: "https://github.com/minseokk77/vesper-dsp/releases/latest",
  woofer: "https://github.com/minseokk77/vesper-woofer/releases/latest",
};

type ProductKey = keyof typeof RELEASES;

function BrandMark() {
  return (
    <span className="brand-mark" aria-hidden="true">
      <i />
      <i />
      <i />
    </span>
  );
}

function SelectField({ value }: { value: string }) {
  return (
    <div className="app-select">
      <span>{value}</span>
      <i aria-hidden="true">⌄</i>
    </div>
  );
}

function WindowActions({ signal = false }: { signal?: boolean }) {
  return (
    <div className="app-window-actions" aria-hidden="true">
      <span className="app-action-glyph">⚙</span>
      {signal && <span className="app-action-glyph app-signal-glyph">▥</span>}
      <span className="app-window-dot" />
      <span className="app-window-dot" />
    </div>
  );
}

function OutputActions() {
  return (
    <span className="app-output-actions" aria-hidden="true">
      <i className="app-output-signal">▥</i>
      <i className="app-output-settings">⚙</i>
      <i className="app-output-power">⏻</i>
    </span>
  );
}

function ActualAppScreens({
  language,
  product,
}: {
  language: Language;
  product: ProductKey;
}) {
  const isKorean = language === "ko";

  return (
    <div
      className="actual-apps"
      aria-label={
        isKorean
          ? `Vesper ${product === "dsp" ? "DSP" : "Woofer"} 실제 앱 화면`
          : `Actual Vesper ${product === "dsp" ? "DSP" : "Woofer"} application screen`
      }
    >
      <figure className="actual-app actual-app-dsp" hidden={product !== "dsp"}>
        <div className="app-ui app-ui-dsp">
          <div className="app-titlebar">
            <h3>
              Vesper <span>|</span> DSP
            </h3>
            <div className="app-status active">
              <i />
              ACTIVE
            </div>
            <WindowActions signal />
          </div>

          <div className="app-fields">
            <label>
              INPUT SOURCE
              <SelectField value="기본 입력 장치" />
            </label>
            <label>
              OUTPUT DEVICE
              <SelectField value="기본 출력 장치" />
            </label>
          </div>

          <section className="app-panel">
            <h4>
              <span className="app-blue">▥</span> DSP 엔진 설정
            </h4>
            <div className="app-setting">
              <div>
                <strong>샘플 레이트 변환</strong>
                <small>출력 샘플 속도 관리 방법</small>
              </div>
              <SelectField value="최대 PCM 레이트 우선" />
            </div>
            <div className="app-setting">
              <strong>리샘플링 필터</strong>
              <SelectField value="정확한 최소 단계" />
            </div>
            <div className="app-setting separated">
              <strong>DSD ▶ PCM 필터</strong>
              <SelectField value="권장함 (30kHz Low-pass)" />
            </div>
            <div className="app-setting">
              <strong>DSD ▶ PCM 게인</strong>
              <SelectField value="+6.0 dB" />
            </div>
          </section>

          <section className="app-panel app-auto-eq">
            <h4 className="app-blue">
              자동 EQ <small>[이어폰] 모델 데이터베이스</small>
              <b aria-hidden="true">↯</b>
            </h4>
            <div className="app-search">
              <span>영문 모델명 통합 검색 (스피커/이어폰)</span>
              <b>검색</b>
            </div>
          </section>

          <section className="app-range">
            <div>
              <span>HEADROOM</span>
              <strong>−3.0 <small>dB</small></strong>
            </div>
            <i style={{ "--range": "74%" } as CSSProperties} />
            <label className="app-checkbox">
              <i>✓</i> 클리핑 감지
            </label>
          </section>
          <div className="app-button-row">
            <button className="app-engine-button" type="button">STOP ENGINE</button>
            <button className="app-mute-button" type="button">MUTE</button>
          </div>
        </div>
      </figure>
      <figure className="actual-app actual-app-woofer" hidden={product !== "woofer"}>
        <div className="app-ui app-ui-woofer">
          <div className="app-titlebar">
            <h3>
              Vesper <span>|</span> Woofer
            </h3>
            <div className="app-status active">
              <i />
              ACTIVE
            </div>
            <WindowActions />
          </div>

          <div className="device-status">
            <i />
            <div>
              <strong>Audio Device</strong>
              <small>Ready Mode</small>
            </div>
            <div>
              <strong>384 kHz</strong>
              <small>Sample Rate</small>
            </div>
          </div>

          <div className="app-presets">
            <span>🎬 Movie</span>
            <strong>🎵 Music</strong>
            <span>🎮 Gaming</span>
            <b>SAVE</b>
          </div>

          <div className="app-fields woofer-fields">
            <label>
              AUDIO SOURCE (VIRTUAL CABLE)
              <SelectField value="기본 입력 장치" />
            </label>
            <label className="app-output-field">
              <span className="app-output-label">
                PRIMARY EARPHONES
                <OutputActions />
              </span>
              <SelectField value="메인 출력 선택" />
            </label>
            <label className="app-output-field">
              <span className="app-output-label">
                SUB WOOFER
                <OutputActions />
              </span>
              <SelectField value="서브우퍼 출력 선택" />
            </label>
          </div>

          <section className="app-range">
            <div>
              <span>PHASE DELAY</span>
              <strong>230 <small>ms</small></strong>
            </div>
            <i style={{ "--range": "24%" } as CSSProperties} />
          </section>
          <section className="app-range app-crossover">
            <div>
              <span>CROSSOVER (SUB WOOFER)</span>
              <small className="app-slope-label">SLOPE</small>
              <SelectField value="24 dB/Octave" />
            </div>
            <div className="app-lowpass">
              <span>Low Pass Filter</span>
              <strong>100 Hz</strong>
            </div>
            <i style={{ "--range": "38%" } as CSSProperties} />
            <small>지정한 주파수보다 낮은 저음역대만 우퍼로 통과시킵니다.</small>
          </section>
          <button className="app-engine-button danger" type="button">
            STOP ENGINE
          </button>
        </div>
      </figure>
    </div>
  );
}

export default function Home() {
  const [language, setLanguage] = useState<Language>("ko");
  const [activeProduct, setActiveProduct] = useState<ProductKey>("dsp");
  const copy = content[language];
  const selectedProduct = copy.products[activeProduct];

  useEffect(() => {
    document.documentElement.lang = language;
    document.title =
      language === "ko"
        ? "Vesper | Windows 오디오를 더 정교하게"
        : "Vesper | Refined audio for Windows";
  }, [language]);

  const toggleLanguage = () => {
    setLanguage((current) => (current === "ko" ? "en" : "ko"));
  };

  return (
    <>
      <a className="skip-link" href="#main-content">
        {copy.skip}
      </a>

      <header className="site-header">
        <nav className="nav-shell glass" aria-label={copy.navLabel}>
          <a className="brand" href="#top" aria-label={copy.homeLabel}>
            <BrandMark />
            Vesper
          </a>
          <div className="nav-links">
            <a href="#products">{copy.nav.products}</a>
            <a href="#experience">{copy.nav.features}</a>
            <a href="#faq">{copy.nav.faq}</a>
          </div>
          <button
            className="nav-language"
            type="button"
            aria-label={copy.switchLabel}
            onClick={toggleLanguage}
          >
            <span>{language.toUpperCase()}</span>
            <i aria-hidden="true">⇄</i>
            <strong>{copy.switchText}</strong>
          </button>
        </nav>
      </header>

      <main id="main-content">
        <section className="hero" id="top">
          <div className="ambient ambient-one" />
          <div className="ambient ambient-two" />

          <div className="hero-layout section-shell">
            <div className="hero-copy">
              <div className="availability">
                <i />
                {copy.hero.availability}
              </div>
              <h1>
                {copy.hero.titleLineOne}
                <br />
                <span>{copy.hero.titleLineTwo}</span>
              </h1>
              <p>
                {copy.hero.descriptionLineOne}
                <br />
                {copy.hero.descriptionLineTwo}
              </p>
              <div className="hero-actions">
                <a className="button primary" href={RELEASES[activeProduct]}>
                  {selectedProduct.action}
                  <span aria-hidden="true">↗</span>
                </a>
                <a className="button secondary" href="#products">
                  {copy.hero.primaryAction}
                  <span aria-hidden="true">↓</span>
                </a>
              </div>
              <small className="hero-note">{copy.hero.note}</small>
            </div>

            <div className="hero-product-stage">
              <div
                className="product-switcher"
                role="tablist"
                aria-label={
                  language === "ko" ? "Vesper 제품 선택" : "Choose a Vesper product"
                }
              >
                {(["dsp", "woofer"] as const).map((product) => (
                  <button
                    key={product}
                    type="button"
                    role="tab"
                    aria-selected={activeProduct === product}
                    className={activeProduct === product ? "active" : ""}
                    onClick={() => setActiveProduct(product)}
                  >
                    <span>Vesper</span>
                    <strong>{product === "dsp" ? "DSP" : "Woofer"}</strong>
                  </button>
                ))}
              </div>
              <div className="selected-product-copy">
                <span>
                  {activeProduct === "dsp" ? "HEADPHONES + SPEAKERS" : "2.1 CHANNEL"}
                </span>
                <p>{selectedProduct.category}</p>
              </div>
              <div className="hero-visual">
                <div className="window-glow" />
                <ActualAppScreens
                  language={language}
                  product={activeProduct}
                />
                <div className="preview-fade" aria-hidden="true" />
              </div>
            </div>
          </div>
        </section>

        <section
          className="statement"
          aria-label={copy.statement.ariaLabel}
        >
          <p className="section-kicker">{copy.statement.label}</p>
          <h2>
            {copy.statement.lineOne}
            <br />
            <span>{copy.statement.lineTwo}</span>
          </h2>
        </section>

        <section className="products section-shell" id="products">
          <article className="product-card dsp-card">
            <div className="card-orb orb-blue" />
            <div className="product-top">
              <span className="product-icon">⌁</span>
              <span className="product-badge">HEADPHONES + SPEAKERS</span>
            </div>
            <div className="product-copy">
              <p>{copy.products.dsp.category}</p>
              <h2>Vesper DSP</h2>
              <span>{copy.products.dsp.description}</span>
            </div>
            <ul className="feature-list">
              {copy.products.dsp.features.map((feature) => (
                <li key={feature}>{feature}</li>
              ))}
            </ul>
            <a href={RELEASES.dsp}>
              {copy.products.dsp.action} <span aria-hidden="true">↗</span>
            </a>
          </article>

          <article className="product-card woofer-card">
            <div className="card-orb orb-violet" />
            <div className="product-top">
              <span className="product-icon">◉</span>
              <span className="product-badge">2.1 CHANNEL</span>
            </div>
            <div className="product-copy">
              <p>{copy.products.woofer.category}</p>
              <h2>Vesper Woofer</h2>
              <span>{copy.products.woofer.description}</span>
            </div>
            <ul className="feature-list">
              {copy.products.woofer.features.map((feature) => (
                <li key={feature}>{feature}</li>
              ))}
            </ul>
            <a href={RELEASES.woofer}>
              {copy.products.woofer.action} <span aria-hidden="true">↗</span>
            </a>
          </article>
        </section>

        <section className="experience section-shell" id="experience">
          <div className="experience-heading">
            <p className="section-kicker">{copy.experience.label}</p>
            <h2>
              {copy.experience.titleLineOne}
              <br />
              {copy.experience.titleLineTwo}
            </h2>
            <p>{copy.experience.description}</p>
          </div>

          <div className="feature-grid">
            {copy.experience.features.map((feature) => (
              <article className="feature-card glass" key={feature.title}>
                <span className="feature-icon">{feature.icon}</span>
                <h3>{feature.title}</h3>
                <p>{feature.description}</p>
              </article>
            ))}
          </div>
        </section>

        <section className="setup section-shell">
          <div className="setup-card glass">
            <div className="setup-copy">
              <p className="section-kicker">{copy.setup.label}</p>
              <h2>{copy.setup.title}</h2>
              <p>{copy.setup.description}</p>
              <a href="#products">{copy.setup.action} ↓</a>
            </div>
            <ol className="setup-steps">
              {copy.setup.steps.map((step, index) => (
                <li key={step.title}>
                  <span>{index + 1}</span>
                  <div>
                    <strong>{step.title}</strong>
                    <small>{step.description}</small>
                  </div>
                </li>
              ))}
            </ol>
          </div>
        </section>

        <section className="faq section-shell" id="faq">
          <div>
            <p className="section-kicker">{copy.faq.label}</p>
            <h2>{copy.faq.title}</h2>
          </div>
          <div className="faq-list">
            {copy.faq.items.map((faq) => (
              <details key={faq.question}>
                <summary>
                  {faq.question}
                  <span aria-hidden="true">+</span>
                </summary>
                <p>{faq.answer}</p>
              </details>
            ))}
          </div>
        </section>

        <section className="final-cta section-shell">
          <div className="cta-glow" />
          <BrandMark />
          <p>{copy.final.label}</p>
          <h2>{copy.final.title}</h2>
          <a className="button primary" href="#products">
            {copy.final.action}
            <span aria-hidden="true">↑</span>
          </a>
        </section>
      </main>

      <footer className="site-footer section-shell">
        <a className="brand" href="#top">
          <BrandMark />
          Vesper
        </a>
        <p>{copy.footer}</p>
        <div>
          <a href="https://github.com/minseokk77/vesper-dsp">DSP GitHub</a>
          <a href="https://github.com/minseokk77/vesper-woofer">
            Woofer GitHub
          </a>
        </div>
      </footer>
    </>
  );
}
