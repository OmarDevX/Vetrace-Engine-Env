import { CATEGORIES, EXAMPLES } from "./examples-data.js";

const grid = document.querySelector("[data-example-grid]");
const count = document.querySelector("[data-example-count]");
const search = document.querySelector("[data-example-search]");
const categories = document.querySelector("[data-category-list]");
const empty = document.querySelector("[data-empty-state]");
let activeCategory = "All";
let query = "";

function card(example) {
  const tags = example.tags.map((tag) => `<span>${tag}</span>`).join("");
  return `
    <a class="example-card" href="./play.html?example=${encodeURIComponent(example.slug)}" data-example-card>
      <div class="example-preview preview-${example.slug}" aria-hidden="true">
        <div class="preview-grid"></div>
        <div class="preview-object"></div>
        <span class="live-pill"><i></i> Live WASM</span>
      </div>
      <div class="example-card-body">
        <div class="example-card-meta">
          <span>${example.category}</span>
          <span>${example.complexity}</span>
        </div>
        <h2>${example.title}</h2>
        <p>${example.description}</p>
        <div class="tag-row">${tags}</div>
        <span class="card-action">Run example <b aria-hidden="true">→</b></span>
      </div>
    </a>`;
}

function render() {
  const needle = query.trim().toLowerCase();
  const visible = EXAMPLES.filter((example) => {
    const categoryMatches = activeCategory === "All" || example.category === activeCategory;
    const haystack = [example.title, example.description, example.category, ...example.tags]
      .join(" ")
      .toLowerCase();
    return categoryMatches && (!needle || haystack.includes(needle));
  });
  grid.innerHTML = visible.map(card).join("");
  count.textContent = `${visible.length} example${visible.length === 1 ? "" : "s"}`;
  empty.hidden = visible.length !== 0;
}

categories.innerHTML = CATEGORIES.map((category) => `
  <button class="category-button${category === activeCategory ? " active" : ""}" data-category="${category}">
    <span>${category}</span>
    <b>${category === "All" ? EXAMPLES.length : EXAMPLES.filter((item) => item.category === category).length}</b>
  </button>`).join("");

categories.addEventListener("click", (event) => {
  const button = event.target.closest("[data-category]");
  if (!button) return;
  activeCategory = button.dataset.category;
  categories.querySelectorAll(".category-button").forEach((item) => {
    item.classList.toggle("active", item === button);
  });
  render();
});

search.addEventListener("input", () => {
  query = search.value;
  render();
});

render();
