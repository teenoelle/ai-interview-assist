/** Svelte action: renders a fixed-position tooltip above the element, clamped to viewport. */
export function tooltip(node: HTMLElement, text: string | undefined) {
  let div: HTMLDivElement | null = null;

  function show() {
    if (!text) return;
    div = document.createElement('div');
    div.className = 'app-tooltip';
    div.textContent = text;
    document.body.appendChild(div);
    requestAnimationFrame(() => {
      if (!div) return;
      const r = node.getBoundingClientRect();
      const tw = div.offsetWidth;
      const th = div.offsetHeight;
      let left = r.left + r.width / 2 - tw / 2;
      const top = r.top - th - 6;
      left = Math.max(8, Math.min(left, window.innerWidth - tw - 8));
      div.style.left = `${left}px`;
      div.style.top = `${Math.max(4, top)}px`;
      div.style.opacity = '1';
    });
  }

  function hide() {
    div?.remove();
    div = null;
  }

  node.addEventListener('mouseenter', show);
  node.addEventListener('mouseleave', hide);
  node.addEventListener('focus', show);
  node.addEventListener('blur', hide);

  return {
    update(newText: string | undefined) {
      text = newText;
      if (div && text) div.textContent = text;
      if (div && !text) hide();
    },
    destroy() {
      hide();
      node.removeEventListener('mouseenter', show);
      node.removeEventListener('mouseleave', hide);
      node.removeEventListener('focus', show);
      node.removeEventListener('blur', hide);
    },
  };
}
