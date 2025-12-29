  
    let params = new URLSearchParams(document.location.search);
    let value = params.get('search'); 
    const input = document.getElementById('query');
    input.value = value || ""
    const url = window.location.origin + window.location.pathname
    input.addEventListener('keydown', function(event) {
    if (event.key === 'Enter') {
        const query = input.value.trim();

        if (query !== NaN) {
            window.location.href = `${url}?search=${encodeURIComponent(query)}`;
        }
    }
  });

    //deleteAllCookies()
    const b = document.body;
    b.dataset.theme = getThemeCookie();
    setThemeCookie(b.dataset.theme);
    var checkbox = document.getElementById("toggle");
    let st = true;
    if (b.dataset.theme=="light"){
      st=false;
    }
    checkbox.checked = st;

    checkbox.addEventListener('change', function() {toggleTheme();})
    function toggleTheme() {
      
      b.dataset.theme = b.dataset.theme === "light" ? "dark" : "light";
      setThemeCookie(b.dataset.theme);
    }
    function setThemeCookie(theme) {
      const days = 30;
      const expires = new Date(Date.now() + days * 864e5).toUTCString();
      document.cookie = `theme=${theme}; expires=${expires}; path=/`;
    }
    function getThemeCookie() {
      const match = document.cookie.match(/(^|;) ?theme=([^;]*)/);
      return match ? match[2] : "dark";
    }
    function deleteAllCookies() {
        document.cookie.split(';').forEach(cookie => {
        const eqPos = cookie.indexOf('=');
        const name = eqPos > -1 ? cookie.substring(0, eqPos) : cookie;
        document.cookie = name + '=;expires=Thu, 01 Jan 1970 00:00:00 GMT';
    });
}
