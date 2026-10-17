import { useEffect } from 'react'
import { ActionManager } from '../../common/components/ActionManager'
import { Window } from '../components/Window'
import { trackEvent } from '../../common/track'

export function ActionManagerWindow() {
    useEffect(() => {
        trackEvent('screen_view', { name: 'ActionManager' })
    }, [])

    return (
        <Window>
            <ActionManager />
        </Window>
    )
}
