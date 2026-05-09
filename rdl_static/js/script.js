      
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
      function deleteAllCookies() {
        document.cookie.split(';').forEach(cookie => {
        const eqPos = cookie.indexOf('=');
        const name = eqPos > -1 ? cookie.substring(0, eqPos) : cookie;
        document.cookie = name + '=;expires=Thu, 01 Jan 1970 00:00:00 GMT';
    });}