// Classic, blocking head script: apply the preference before styles can paint.
(() => {
  const key = "buzIt.theme";
  let theme = "light";
  try {
    if (localStorage.getItem(key) === "dark") theme = "dark";
  } catch {
    /* Storage may be unavailable in private contexts. */
  }
  const apply = (value) => {
    theme = value === "dark" ? "dark" : "light";
    document.documentElement.dataset.theme = theme;
    const toggle = document.getElementById("theme-toggle");
    if (toggle) {
      toggle.setAttribute("aria-checked", String(theme === "dark"));
      toggle.textContent = theme === "dark" ? "Dark" : "Light";
    }
  };
  apply(theme);
  document.addEventListener("DOMContentLoaded", () => {
    apply(theme);
    document.getElementById("theme-toggle").addEventListener("click", () => {
      apply(theme === "dark" ? "light" : "dark");
      try {
        localStorage.setItem(key, theme);
      } catch {
        /* The switch still works for this page. */
      }
    });
  });
  window.addEventListener("storage", (event) => {
    if (event.key === key || event.key === null) apply(event.newValue);
  });
})();
