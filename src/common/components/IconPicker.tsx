import { IconType } from 'react-icons'
import { createUseStyles } from 'react-jss'
import { Button } from 'baseui-sd/button'
import { useEffect, useState, useMemo } from 'react'
import { VariableSizeGrid as Grid } from 'react-window'
import { Input } from 'baseui-sd/input'
import { useTranslation } from 'react-i18next'
import { ActionIcon } from './ActionIcon'

const useStyles = createUseStyles({
    root: {},
    icons: {
        padding: '20px 5px',
    },
    iconSearchBar: {
        display: 'flex',
        flexDirection: 'row',
        alignItems: 'center',
        gap: '10px',
        marginBottom: '10px',
    },
    iconItem: {
        display: 'inline-flex',
    },
})

export interface IIconPickerProps {
    value?: string
    onChange?: (value: string) => void
}

export function IconPicker({ value, onChange }: IIconPickerProps) {
    const { t } = useTranslation()
    const [showIcons, setShowIcons] = useState(false)
    const [currentValue, setCurrentValue] = useState(value ?? 'MdMusicVideo')

    useEffect(() => {
        if (value) {
            setCurrentValue(value)
        }
    }, [value])

    useEffect(() => {
        onChange?.(currentValue)
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [currentValue])

    const [searchInputText, setSearchInputText] = useState('')

    const [searchText, setSearchText] = useState('')

    // The full Material icon set is a ~2MB chunk; only fetch it when the
    // picker grid is actually expanded.
    const [iconMap, setIconMap] = useState<Record<string, IconType>>()
    useEffect(() => {
        if (!showIcons || iconMap) {
            return
        }
        import('./md-icon-registry').then((m) => setIconMap(m.mdIconMap))
    }, [showIcons, iconMap])

    const iconNames = useMemo(() => {
        const names = Object.keys(iconMap ?? {})
        if (searchText) {
            return names.filter((name) => name.toLowerCase().includes(searchText.toLowerCase()))
        }
        return names
    }, [searchText, iconMap])

    const styles = useStyles()

    return (
        <div className={styles.root}>
            <Button
                size='mini'
                kind='primary'
                onClick={(e) => {
                    e.stopPropagation()
                    e.preventDefault()
                    setShowIcons((v) => !v)
                }}
            >
                <ActionIcon name={currentValue} size={16} />
            </Button>
            {showIcons && (
                <div className={styles.icons}>
                    <div className={styles.iconSearchBar}>
                        <Input
                            overrides={{
                                Root: {
                                    style: {
                                        width: '200px',
                                    },
                                },
                            }}
                            value={searchInputText}
                            onChange={(e) => {
                                setSearchInputText(e.currentTarget.value)
                            }}
                            onKeyPress={(e) => {
                                e.stopPropagation()
                                if (e.key === 'Enter') {
                                    setSearchText(searchInputText)
                                    e.preventDefault()
                                }
                            }}
                            placeholder='Search Icon'
                            size='mini'
                        />
                        <Button
                            size='mini'
                            kind='secondary'
                            onClick={(e) => {
                                e.stopPropagation()
                                e.preventDefault()
                                setSearchText(searchInputText)
                            }}
                        >
                            {t('Search')}
                        </Button>
                    </div>
                    <Grid
                        height={250}
                        width={400}
                        rowHeight={() => 40}
                        columnWidth={() => 40}
                        rowCount={Math.round(iconNames.length / 10)}
                        columnCount={10}
                    >
                        {({ columnIndex, rowIndex, style }) => {
                            const key = iconNames[rowIndex * 10 + columnIndex]
                            if (!key) {
                                return null
                            }
                            const Icon = (iconMap ?? {})[key] as IconType
                            return (
                                <div key={key} style={style}>
                                    <Button
                                        kind={currentValue === key ? 'primary' : 'secondary'}
                                        size='mini'
                                        onClick={(e) => {
                                            e.stopPropagation()
                                            e.preventDefault()
                                            setCurrentValue(key)
                                            setShowIcons(false)
                                        }}
                                    >
                                        <Icon size={16} />
                                    </Button>
                                </div>
                            )
                        }}
                    </Grid>
                </div>
            )}
        </div>
    )
}
