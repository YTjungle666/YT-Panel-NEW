import { ss } from '@/utils/storage'
import { PanelPanelConfigStyleEnum, PanelStateNetworkModeEnum } from '@/enums'
import defaultBackground from '@/assets/defaultBackground.webp'
const LOCAL_NAME = 'panelStorage'

const defaultFooterHtml = ''

export function defaultStatePanelConfig(): Panel.panelConfig {
  return {
    backgroundImageSrc: defaultBackground,
    backgroundBlur: 0,
    backgroundMaskNumber: 0,
    iconStyle: PanelPanelConfigStyleEnum.icon,
    iconTextColor: '#ffffff',
    iconTextInfoHideDescription: false,
    iconTextIconHideTitle: false,
    logoText: 'YT-Panel',
    logoImageSrc: '',
    clockShowSecond: true,
    searchBoxShow: true,
    searchBoxSearchIcon: true,
    marginBottom: 10,
    marginTop: 10,
    maxWidth: 1200,
    maxWidthUnit: 'px',
    marginX: 5,
    footerHtml: defaultFooterHtml,
    systemMonitorShow: false,
    systemMonitorShowTitle: true,
    systemMonitorPublicVisitModeShow: false,
    netModeChangeButtonShow: true,
    autoNetworkWallpaper: false, // 默认关闭自动获取网络壁纸
    autoNetworkWallpaperApi: 'https://img.xjh.me/random_img.php?return=302&type=bg&ctype=nature', // 默认自动获取网络壁纸API地址

  }
}

export function defaultState(): Panel.State {
  return {
    rightSiderCollapsed: false,
    leftSiderCollapsed: false,
    networkMode: PanelStateNetworkModeEnum.wan,
    panelConfig: { ...defaultStatePanelConfig() },
  }
}

export function getLocalState(): Panel.State {
  const localState = ss.get(LOCAL_NAME)
  return { ...defaultState(), ...localState }
}

export function setLocalState(state: Panel.State) {
  ss.set(LOCAL_NAME, state)
}

export function removeLocalState() {
  ss.remove(LOCAL_NAME)
}
