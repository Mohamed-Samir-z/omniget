import { describe, expect, it } from "vitest";
import fs from "node:fs";

const page = fs.readFileSync("src/routes/about/+page.svelte", "utf8");
const layout = fs.readFileSync("src/routes/about/+layout.svelte", "utf8");

describe("Android About identity and navigation", () => {
  it("keeps the user's top navigation on the overview and subpages", () => {
    for (const href of ["/about", "/about/changelog", "/about/project", "/about/terms"]) {
      expect(layout).toContain(`href: "${href}"`);
    }
    expect(layout).not.toContain("display: none");
    expect(layout).not.toContain("flex-wrap: wrap");
    expect(layout).toContain('aria-current={isActive(tab.href) ? "page" : undefined}');
  });

  it("uses the approved portrait and the existing fixed native contact action", () => {
    expect(page).toContain('src="/developer/mohamed.jpg"');
    expect(page).toContain("await openDeveloperContact()");
    expect(page).toContain("if (contactBusy) return");
    expect(page).toContain("aria-busy={contactBusy}");
    expect(page).not.toContain("plugin-opener");
  });

  it("keeps original-project attribution and all original external destinations", () => {
    for (const value of ["LibreGet", "OpenSelena", "https://github.com/OpenSelena/libreget", "https://github.com/OpenSelena", "https://t.me/OpenSelena", '$t("about.credit")']) {
      expect(page).toContain(value);
    }
    expect(page).toContain("original-project");
    expect(page).toContain("android-identity");
  });

  it("retains debug access only when enabled", () => {
    expect(page).toContain("isDebugEnabled() ?");
    expect(page).toContain('href: "/about/debug"');
    expect(layout).toContain("isDebugEnabled() ? ALL_TABS");
  });

  it("provides all nine Android-edition strings in every locale", () => {
    const locales = fs.readdirSync("src/lib/i18n").filter(name => name.endsWith(".json"));
    expect(locales).toHaveLength(13);
    const expected = ["name", "role", "heading", "detail", "contact", "original", "based", "github", "portrait"].sort();
    for (const locale of locales) {
      const strings = JSON.parse(fs.readFileSync(`src/lib/i18n/${locale}`, "utf8")).about.android_edition;
      expect(Object.keys(strings).sort(), locale).toEqual(expected);
      for (const value of Object.values(strings)) expect(String(value).trim().length, locale).toBeGreaterThan(0);
      expect(strings.detail, locale).toContain("omniget");
      expect(strings.based, locale).toContain("LibreGet");
    }
  });
});
