// allow mdbook scripts to load first
window.addEventListener("load", () => {
  const sidebarNode = document.querySelector("mdbook-sidebar-scrollbox");
  const searchNode = document.querySelector("#mdbook-search-wrapper");
  const searchToggleNode = document.querySelector("#mdbook-search-toggle");

  // move search bar to the sidebar
  if (sidebarNode && searchNode) {
    sidebarNode.prepend(searchNode);
  }

  // first click simulates initializing search
  searchToggleNode.click();
  // second simulated click enables user's initial click of search toggle
  searchToggleNode.click();

  // remove focus from the search input
  setTimeout(() => searchNode.querySelector("input").blur(), 50);
});
