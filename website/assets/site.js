const navToggle = document.querySelector("[data-nav-toggle]");
const nav = document.querySelector("[data-nav]");

navToggle?.addEventListener("click", () => {
  const open = nav?.classList.toggle("is-open") ?? false;
  navToggle.setAttribute("aria-expanded", String(open));
});

document.querySelectorAll("[data-copy]").forEach((button) => {
  button.addEventListener("click", async () => {
    const selector = button.getAttribute("data-copy");
    const source = selector ? document.querySelector(selector) : null;
    if (!source) return;
    await navigator.clipboard.writeText(source.textContent.trim());
    const original = button.textContent;
    button.textContent = "Copied";
    setTimeout(() => { button.textContent = original; }, 1400);
  });
});

document.querySelectorAll("[data-year]").forEach((node) => {
  node.textContent = String(new Date().getFullYear());
});
