// Overline website: scroll reveals, and clips of the real app that play only while they're on
// screen (and stay still for people who prefer less motion).
(function () {
  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (reduce)
    document.querySelectorAll(".hero-video").forEach((v) => {
      v.removeAttribute("autoplay");
      v.pause();
    });

  // ── Scroll reveal.
  const reveals = document.querySelectorAll(".reveal");
  if ("IntersectionObserver" in window && !reduce) {
    const io = new IntersectionObserver(
      (entries) =>
        entries.forEach((e) => {
          if (e.isIntersecting) {
            e.target.classList.add("in");
            io.unobserve(e.target);
          }
        }),
      { rootMargin: "0px 0px -8% 0px", threshold: 0.12 },
    );
    reveals.forEach((el) => io.observe(el));
  } else {
    reveals.forEach((el) => el.classList.add("in"));
  }

  // ── Feature clips: load when near, play only while visible (saves data and battery).
  const clips = document.querySelectorAll(".feature-media video[data-src]");
  if ("IntersectionObserver" in window) {
    const vio = new IntersectionObserver(
      (entries) =>
        entries.forEach((e) => {
          const v = e.target;
          if (e.isIntersecting) {
            if (!v.src) v.src = v.dataset.src;
            if (!reduce) v.play().catch(() => {});
          } else {
            v.pause();
          }
        }),
      { threshold: 0.35 },
    );
    clips.forEach((v) => vio.observe(v));
  } else {
    clips.forEach((v) => (v.src = v.dataset.src));
  }

  // ── The film plays when you ask for it ("Watch the film").
  const film = document.getElementById("film-video");
  document
    .querySelectorAll('a[href="#film"]')
    .forEach((a) => a.addEventListener("click", () => setTimeout(() => film.play().catch(() => {}), 600)));
})();
