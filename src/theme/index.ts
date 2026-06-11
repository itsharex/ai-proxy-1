import type { GlobalThemeOverrides } from 'naive-ui'

const fontFamily = `'DM Sans', -apple-system, 'PingFang SC', 'Noto Sans SC', 'Microsoft YaHei', sans-serif`
const fontMono = `'JetBrains Mono', 'SF Mono', 'Menlo', monospace`

export const lightThemeOverrides: GlobalThemeOverrides = {
  common: {
    fontFamily,
    fontFamilyMono: fontMono,
    borderRadius: '6px',
    borderRadiusSmall: '4px',
    primaryColor: '#0891B2',
    primaryColorHover: '#0E7490',
    primaryColorPressed: '#155E75',
    primaryColorSuppl: '#0891B2',
    bodyColor: '#F8F9FB',
    cardColor: '#FFFFFF',
    modalColor: '#FFFFFF',
    popoverColor: '#FFFFFF',
    borderColor: '#E2E6EB',
    dividerColor: '#ECF0F4',
    textColor1: '#141720',
    textColor2: '#5C6370',
    textColor3: '#9BA3B1',
    inputColor: '#F1F3F5',
    tableColor: '#FFFFFF',
    tableColorHover: '#F8F9FB',
    hoverColor: '#F1F3F5',
    successColor: '#16A34A',
    warningColor: '#D97706',
    errorColor: '#DC2626',
    infoColor: '#6366F1',
  },
  Card: {
    borderRadius: '8px',
    borderColor: '#E2E6EB',
    color: '#FFFFFF',
    titleTextColor: '#141720',
  },
  Button: {
    borderRadiusMedium: '6px',
    borderRadiusSmall: '4px',
  },
  DataTable: {
    thColor: '#F1F3F5',
    thTextColor: '#5C6370',
    thFontWeight: '500',
    tdColor: '#FFFFFF',
    tdColorHover: '#F8F9FB',
    borderColor: '#ECF0F4',
    borderRadius: '8px',
    fontSize: '13px',
  },
  Menu: {
    borderRadius: '6px',
    itemColorActive: '#E0F7FA',
    itemTextColorActive: '#0891B2',
    itemIconColorActive: '#0891B2',
    itemColorActiveHover: '#E0F7FA',
    itemTextColorActiveHover: '#0891B2',
    itemIconColorActiveHover: '#0891B2',
    itemColorHover: '#F1F3F5',
    itemTextColorHover: '#141720',
    itemIconColorHover: '#141720',
    itemHeight: '38px',
  },
  Tag: {
    borderRadius: '4px',
  },
  Input: {
    borderRadius: '6px',
    color: '#F1F3F5',
    borderFocus: '1px solid #0891B2',
    borderHover: '1px solid #0E7490',
  },
  Switch: {
    railColorActive: '#0891B2',
  },
  Modal: {
    borderRadius: '10px',
  },
  Tabs: {
    tabTextColorActiveLine: '#0891B2',
    tabTextColorHoverLine: '#0E7490',
    barColor: '#0891B2',
  },
  Radio: {
    buttonColorActive: '#0891B2',
    buttonTextColorActive: '#FFFFFF',
  },
  Form: {
    labelTextColor: '#5C6370',
    feedbackTextColor: '#9BA3B1',
  },
}

export const darkThemeOverrides: GlobalThemeOverrides = {
  common: {
    fontFamily,
    fontFamilyMono: fontMono,
    borderRadius: '6px',
    borderRadiusSmall: '4px',
    primaryColor: '#22D3EE',
    primaryColorHover: '#06B6D4',
    primaryColorPressed: '#67E8F9',
    primaryColorSuppl: '#22D3EE',
    bodyColor: '#0D1017',
    cardColor: '#161A24',
    modalColor: '#161A24',
    popoverColor: '#1C2130',
    borderColor: '#252B38',
    dividerColor: '#1E2330',
    textColor1: '#E4E8F0',
    textColor2: '#8892A2',
    textColor3: '#555E6E',
    inputColor: '#111520',
    tableColor: '#161A24',
    tableColorHover: '#1C2130',
    hoverColor: '#1C2130',
    successColor: '#22C55E',
    warningColor: '#F59E0B',
    errorColor: '#EF4444',
    infoColor: '#818CF8',
  },
  Card: {
    borderRadius: '8px',
    borderColor: '#252B38',
    color: '#161A24',
    titleTextColor: '#E4E8F0',
  },
  Button: {
    borderRadiusMedium: '6px',
    borderRadiusSmall: '4px',
  },
  DataTable: {
    thColor: '#111520',
    thTextColor: '#8892A2',
    thFontWeight: '500',
    tdColor: '#161A24',
    tdColorHover: '#1C2130',
    borderColor: '#1E2330',
    borderRadius: '8px',
    fontSize: '13px',
  },
  Menu: {
    borderRadius: '6px',
    itemColorActive: '#0C2D3A',
    itemTextColorActive: '#22D3EE',
    itemIconColorActive: '#22D3EE',
    itemColorActiveHover: '#0C2D3A',
    itemTextColorActiveHover: '#22D3EE',
    itemIconColorActiveHover: '#22D3EE',
    itemColorHover: '#1C2130',
    itemTextColorHover: '#E4E8F0',
    itemIconColorHover: '#E4E8F0',
    itemHeight: '38px',
  },
  Tag: {
    borderRadius: '4px',
  },
  Input: {
    borderRadius: '6px',
    color: '#111520',
    borderFocus: '1px solid #22D3EE',
    borderHover: '1px solid #06B6D4',
  },
  Switch: {
    railColorActive: '#22D3EE',
  },
  Modal: {
    borderRadius: '10px',
  },
  Tabs: {
    tabTextColorActiveLine: '#22D3EE',
    tabTextColorHoverLine: '#06B6D4',
    barColor: '#22D3EE',
  },
  Radio: {
    buttonColorActive: '#22D3EE',
    buttonTextColorActive: '#0D1017',
  },
  Form: {
    labelTextColor: '#8892A2',
    feedbackTextColor: '#555E6E',
  },
}

export function getEchartsTheme(isDark: boolean) {
  const text = isDark ? '#8892A2' : '#5C6370'
  const border = isDark ? '#252B38' : '#ECF0F4'
  const bg = isDark ? '#161A24' : '#FFFFFF'
  const palette = isDark
    ? ['#22D3EE', '#818CF8', '#22C55E', '#F59E0B', '#EF4444', '#A78BFA', '#FB923C', '#34D399']
    : ['#0891B2', '#6366F1', '#16A34A', '#D97706', '#DC2626', '#8B5CF6', '#EA580C', '#059669']
  return { text, border, bg, palette }
}
