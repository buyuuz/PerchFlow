/* Shared PerchFlow topbar language chooser.
   The original <select data-lang-select> remains as a working fallback and
   continues to dispatch existing process/task table language handlers. */
document.addEventListener('DOMContentLoaded', () => {
  document.querySelectorAll('[data-pf-language-picker]').forEach(picker => {
    const native = picker.querySelector('select[data-lang-select]');
    const trigger = picker.querySelector('[data-lang-trigger]');
    const menu = picker.querySelector('[data-lang-menu]');
    const label = trigger?.querySelector('[data-lang-current]');
    const code = trigger?.querySelector('[data-lang-code]');
    const options = Array.from(menu?.querySelectorAll('[data-lang-value]') || []);
    if (!native || !trigger || !menu || !label || !code || !options.length) return;

    function selectedLanguage() {
      return typeof getLang === 'function' ? getLang() : native.value;
    }

    function sync() {
      const current = selectedLanguage();
      const selected = options.find(option => option.dataset.langValue === current);
      if (!selected) return;
      native.value = current;
      label.textContent = selected.dataset.langLabel || selectedLanguage();
      code.textContent = selected.dataset.langCode || current.toUpperCase();
      options.forEach(option => {
        option.setAttribute('aria-selected', String(option === selected));
      });
    }

    function close(restoreFocus = false) {
      if (menu.hidden) return;
      menu.hidden = true;
      trigger.setAttribute('aria-expanded', 'false');
      if (restoreFocus) trigger.focus();
    }

    function open(focus = true) {
      menu.hidden = false;
      trigger.setAttribute('aria-expanded', 'true');
      sync();
      if (focus) {
        (options.find(option => option.dataset.langValue === selectedLanguage()) || options[0]).focus();
      }
    }

    trigger.addEventListener('click', () => {
      if (menu.hidden) open();
      else close();
    });

    trigger.addEventListener('keydown', event => {
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        event.preventDefault();
        open();
        if (event.key === 'ArrowUp') options[options.length - 1].focus();
      } else if (event.key === 'Escape' && !menu.hidden) {
        event.preventDefault();
        close(true);
      }
    });

    menu.addEventListener('click', event => {
      const option = event.target.closest('[data-lang-value]');
      if (!option || !menu.contains(option)) return;
      native.value = option.dataset.langValue;
      native.dispatchEvent(new Event('change', {bubbles: true}));
      sync();
      close(true);
    });

    menu.addEventListener('keydown', event => {
      const currentIndex = options.indexOf(document.activeElement);
      let nextIndex = null;
      if (event.key === 'ArrowDown') nextIndex = (currentIndex + 1) % options.length;
      else if (event.key === 'ArrowUp') nextIndex = (currentIndex + options.length - 1) % options.length;
      else if (event.key === 'Home') nextIndex = 0;
      else if (event.key === 'End') nextIndex = options.length - 1;
      else if (event.key === 'Escape') {
        event.preventDefault();
        close(true);
        return;
      } else if (event.key === 'Tab') {
        close();
        return;
      }
      if (nextIndex !== null) {
        event.preventDefault();
        options[nextIndex].focus();
      }
    });

    document.addEventListener('pointerdown', event => {
      if (!picker.contains(event.target)) close();
    });
    document.addEventListener('perchflow:language-changed', sync);
    native.addEventListener('change', sync);

    // Progressive enhancement: display native select unless JS actually starts.
    trigger.hidden = false;
    picker.classList.add('is-enhanced');
    sync();
  });
});
