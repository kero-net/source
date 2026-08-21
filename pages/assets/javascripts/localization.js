(() => {
  "use strict";

  const COOKIE = "template-language";
  const DEFAULT_LOCALE = "en-US";

  function siteRoot() {
    const script = [...document.scripts].find((entry) =>
      entry.src.endsWith("/assets/javascripts/localization.js"),
    );
    return script ? new URL("../../", script.src) : new URL("./", document.baseURI);
  }

  function currentRoute(root) {
    const rootPath = root.pathname.endsWith("/") ? root.pathname : `${root.pathname}/`;
    const path = window.location.pathname;
    return path.startsWith(rootPath) ? path.slice(rootPath.length) : "";
  }

  function readCookie() {
    const prefix = `${COOKIE}=`;
    const value = document.cookie
      .split(";")
      .map((part) => part.trim())
      .find((part) => part.startsWith(prefix));
    return value ? decodeURIComponent(value.slice(prefix.length)) : DEFAULT_LOCALE;
  }

  function writeCookie(locale) {
    document.cookie = `${COOKIE}=${encodeURIComponent(locale)}; Path=${siteRoot().pathname}; Max-Age=31536000; SameSite=Lax`;
  }

  async function applyLocale(locale) {
    const root = siteRoot();
    const route = currentRoute(root);
    const source = new URL(`_locales/${locale}/${route}`, root);
    const response = await fetch(source, { credentials: "same-origin" });
    if (!response.ok) throw new Error(`Unable to load locale ${locale}: ${response.status}`);

    const localized = new DOMParser().parseFromString(await response.text(), "text/html");
    const currentArticle = document.querySelector("article.md-content__inner");
    const localizedArticle = localized.querySelector("article.md-content__inner");
    if (!currentArticle || !localizedArticle) throw new Error(`Locale ${locale} is missing page content`);

    currentArticle.innerHTML = localizedArticle.innerHTML;
    const currentToc = document.querySelector(".md-sidebar--secondary .md-sidebar__inner");
    const localizedToc = localized.querySelector(".md-sidebar--secondary .md-sidebar__inner");
    if (currentToc && localizedToc) currentToc.innerHTML = localizedToc.innerHTML;
    document.documentElement.lang = locale;
    document.title = localized.title;
    writeCookie(locale);
    document.dispatchEvent(new CustomEvent("template:language", { detail: { locale } }));
  }

  function bindLanguageMenu() {
    for (const link of document.querySelectorAll(".md-select__link[hreflang]")) {
      link.addEventListener("click", (event) => {
        event.preventDefault();
        applyLocale(link.hreflang).catch((error) => console.error(error));
      });
    }
  }

  document.addEventListener("DOMContentLoaded", () => {
    bindLanguageMenu();
    const locale = readCookie();
    if (locale !== DEFAULT_LOCALE) {
      applyLocale(locale).catch((error) => {
        console.error(error);
        writeCookie(DEFAULT_LOCALE);
      });
    }
  });
})();
