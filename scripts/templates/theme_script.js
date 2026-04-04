<script>
(() => {
  const root = document.documentElement;
  const storageKey = "grafana-util-docs-theme";
  const select = document.getElementById("theme-select");
  const saved = localStorage.getItem(storageKey) || "auto";
  const applyTheme = (val) => { if(val==="auto") root.removeAttribute("data-theme"); else root.setAttribute("data-theme", val); };
  applyTheme(saved);
  if(select) { select.value = saved; select.onchange = (e) => { localStorage.setItem(storageKey, e.target.value); applyTheme(e.target.value); }; }

  const fontKey = "grafana-util-docs-font";
  const fontSelect = document.getElementById("font-select");
  const savedFont = localStorage.getItem(fontKey) || "1";
  const applyFont = (val) => { root.style.setProperty("--font-scale", val); };
  applyFont(savedFont);
  if(fontSelect) { fontSelect.value = savedFont; fontSelect.onchange = (e) => { localStorage.setItem(fontKey, e.target.value); applyFont(e.target.value); }; }

  const wrapKey = "grafana-util-docs-wrap";
  let isWrapped = localStorage.getItem(wrapKey) === "true";
  const updateWrap = (block, btn, wrapped) => {
    if(!btn) return;
    if(wrapped) { block.classList.add("wrapped"); btn.classList.add("active"); btn.innerText="Wrap: ON"; }
    else { block.classList.remove("wrapped"); btn.classList.remove("active"); btn.innerText="Wrap: OFF"; }
  };
  document.querySelectorAll("pre").forEach(block => {
    const controls = document.createElement("div"); controls.className = "code-controls";
    const wrapBtn = document.createElement("button"); wrapBtn.className = "control-btn";
    updateWrap(block, wrapBtn, isWrapped);
    wrapBtn.onclick = () => { isWrapped = !isWrapped; localStorage.setItem(wrapKey, isWrapped); document.querySelectorAll("pre").forEach(b => updateWrap(b, b.querySelector(".control-btn"), isWrapped)); };
    const copyBtn = document.createElement("button"); copyBtn.className = "control-btn"; copyBtn.innerText = "Copy";
    copyBtn.onclick = () => {
      const raw = block.querySelector("code").innerText;
      navigator.clipboard.writeText(raw).then(() => { copyBtn.innerText="Copied!"; setTimeout(()=>copyBtn.innerText="Copy", 2000); });
    };
    controls.append(wrapBtn, copyBtn); block.append(controls);
  });

  document.querySelectorAll(".nav-group-header").forEach(header => {
    header.onclick = () => { header.closest(".nav-group").classList.toggle("collapsed"); };
  });

  const jumpSelect = document.getElementById("jump-select");
  if(jumpSelect) jumpSelect.onchange = (e) => { if(e.target.value) window.location.href = e.target.value; };
  const pageLocaleSelect = document.getElementById("page-locale-select");
  if(pageLocaleSelect) pageLocaleSelect.onchange = (e) => { if(e.target.value) window.location.href = e.target.value; };

  const landingI18n = document.getElementById("landing-i18n");
  const localeSelect = document.getElementById("locale-select");
  const landingSearchForm = document.getElementById("landing-search-form");
  const landingSearchInput = document.getElementById("landing-search");
  const landingSearchButton = document.getElementById("landing-search-button");
  const landingTitle = document.getElementById("landing-title");
  const landingSummary = document.getElementById("landing-summary");
  const landingSearchHeading = document.getElementById("landing-search-heading");
  const landingSearchCopy = document.getElementById("landing-search-copy");
  const landingSections = document.getElementById("landing-sections");
  const landingMeta = document.getElementById("landing-meta");
  if(landingI18n) {
    const landingData = JSON.parse(landingI18n.textContent);
    const landingLocaleKey = "grafana-util-docs-locale-mode";
    const getBrowserLocale = () => {
      const browserLocales = [...(navigator.languages || []), navigator.language || ""];
      const preferredZh = browserLocales.some((locale) => locale && locale.toLowerCase().startsWith("zh"));
      return preferredZh && landingData["zh-TW"] ? "zh-TW" : "en";
    };
    const readLandingModeFromHash = () => {
      const hash = window.location.hash || "";
      const match = hash.match(/(?:^#|&)lang=([^&]+)/);
      if(!match) return null;
      const value = decodeURIComponent(match[1]);
      return ["auto", "en", "zh-TW"].includes(value) ? value : null;
    };
    const writeLandingModeToHash = (mode) => {
      const url = new URL(window.location.href);
      if(mode === "auto") {
        url.hash = "lang=auto";
      } else {
        url.hash = `lang=${encodeURIComponent(mode)}`;
      }
      window.history.replaceState({}, "", url.toString());
    };
    const pickLandingMode = () => {
      const hashMode = readLandingModeFromHash();
      if(hashMode) return hashMode;
      const saved = localStorage.getItem(landingLocaleKey);
      if(saved && ["auto", "en", "zh-TW"].includes(saved)) return saved;
      return "auto";
    };
    const applyLandingLocale = (mode, {persist = true, syncHash = true} = {}) => {
      const locale = mode === "auto" ? getBrowserLocale() : mode;
      const copy = landingData[locale] || landingData.en;
      if(!copy) return;
      document.documentElement.lang = copy.lang || locale;
      if(localeSelect) localeSelect.value = mode;
      if(landingTitle) landingTitle.textContent = copy.hero_title;
      if(landingSummary) landingSummary.textContent = copy.hero_summary;
      if(landingSearchHeading) landingSearchHeading.textContent = copy.search_heading;
      if(landingSearchCopy) landingSearchCopy.textContent = copy.search_copy;
      if(landingSearchInput) {
        landingSearchInput.placeholder = copy.search_placeholder;
        landingSearchInput.setAttribute("aria-label", copy.search_placeholder);
      }
      if(landingSearchButton) landingSearchButton.textContent = copy.search_button;
      if(landingSections) landingSections.innerHTML = copy.sections_html;
      if(landingMeta) landingMeta.innerHTML = copy.meta_html;
      if(jumpSelect) jumpSelect.innerHTML = copy.jump_options_html;
      if(persist) localStorage.setItem(landingLocaleKey, mode);
      if(syncHash) writeLandingModeToHash(mode);
    };
    applyLandingLocale(pickLandingMode(), {persist: false, syncHash: false});
    const initialMode = pickLandingMode();
    if(localeSelect) localeSelect.value = initialMode;
    writeLandingModeToHash(initialMode);
    if(localeSelect) {
      localeSelect.addEventListener("change", (event) => {
        applyLandingLocale(event.target.value);
      });
    }
    window.addEventListener("hashchange", () => {
      const hashMode = readLandingModeFromHash();
      if(hashMode) applyLandingLocale(hashMode, {persist: true, syncHash: false});
    });
  }

  if(landingSearchForm && landingSearchInput && jumpSelect) {
    landingSearchForm.addEventListener("submit", (event) => {
      event.preventDefault();
      const query = landingSearchInput.value.trim().toLowerCase();
      if(!query) return;
      const options = [...jumpSelect.querySelectorAll("option")].filter((option) => option.value);
      const exact = options.find((option) => option.textContent.toLowerCase().includes(query));
      const fuzzy = options.find((option) => option.value.toLowerCase().includes(query.replace(/\s+/g, "-")));
      const target = exact || fuzzy;
      if(target) window.location.href = target.value;
    });
  }

  const observer = new IntersectionObserver(entries => {
    entries.forEach(entry => {
      const id = entry.target.id; if(!id) return;
      const link = document.querySelector(`.sidebar a[href="#${id}"]`);
      if(link && entry.isIntersecting) { document.querySelectorAll(".sidebar a").forEach(l => l.classList.remove("active")); link.classList.add("active"); }
    });
  }, { rootMargin: "-20px 0px -80% 0px" });
  document.querySelectorAll(".article h2[id], .article h3[id]").forEach(el => observer.observe(el));
})();
</script>
