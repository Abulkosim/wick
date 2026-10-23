const query = new URLSearchParams(location.search)

/** The preview picks a platform with `?platform=windows`; the desktop app reads its WebView. */
export const isWindows = query.get('demo') === '1' || !('__TAURI__' in window)
  ? query.get('platform') === 'windows'
  : navigator.userAgent.includes('Windows')
