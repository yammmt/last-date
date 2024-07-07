function applyTheme() {
    const prefersDarkScheme = window.matchMedia("(prefers-color-scheme: dark)").matches;

    if (prefersDarkScheme) {
        let lights = document.querySelectorAll(".is-light");
        for (let l of lights) {
            l.classList.remove("is-light");
            l.classList.add("is-dark");
        }
    } else {
        let darks = document.querySelectorAll(".is-dark");
        for (let d of darks) {
            d.classList.remove("is-dark");
            d.classList.add("is-light");
        }
    }
}

// https://developer.mozilla.org/en-US/docs/Web/API/Document/DOMContentLoaded_event#checking_whether_loading_is_already_complete
if (document.readyState === "loading") {
    // loading hasn't finished yet
    document.addEventListener("DOMContentLoaded", applyTheme);
} else {
    applyTheme();
}

window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", applyTheme);
