import { darkTheme, lightTheme } from 'naive-ui'

const setupScrollbarStyle = () => {
  const existingStyle = document.getElementById('yt-panel-scrollbar-style')
  if (existingStyle)
    return

  const style = document.createElement('style')
  style.id = 'yt-panel-scrollbar-style'
  const styleContent = `
    ::-webkit-scrollbar {
      background-color: transparent;
      width: ${lightTheme.Scrollbar.common?.scrollbarWidth};
    }
    ::-webkit-scrollbar-thumb {
      background-color: ${lightTheme.Scrollbar.common?.scrollbarColor};
      border-radius: ${lightTheme.Scrollbar.common?.scrollbarBorderRadius};
    }
    html.dark ::-webkit-scrollbar {
      background-color: transparent;
      width: ${darkTheme.Scrollbar.common?.scrollbarWidth};
    }
    html.dark ::-webkit-scrollbar-thumb {
      background-color: ${darkTheme.Scrollbar.common?.scrollbarColor};
      border-radius: ${darkTheme.Scrollbar.common?.scrollbarBorderRadius};
    }
  `

  style.textContent = styleContent
  document.head.appendChild(style)
}

export default setupScrollbarStyle
