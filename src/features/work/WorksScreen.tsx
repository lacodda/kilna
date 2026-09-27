import { Navigate, useNavigate, useParams } from 'react-router'
import { WorkCard } from '@/features/work/WorkCard'

/**
 * An open work, filling the screen. The address carries which one and which
 * tab, so the back button walks between them.
 *
 * Until v0.21 a list of every work sat beside it here, duplicating the
 * catalogue; `/works` with nothing open now sends you to the list that remains.
 */
export function WorksScreen() {
  const navigate = useNavigate()
  const { workId, tab } = useParams()

  if (workId === undefined) return <Navigate to="/catalogue" replace />

  return (
    <WorkCard
      key={workId}
      workId={workId}
      tab={tab}
      onDeleted={() => navigate('/catalogue')}
      onUndone={(restored) => navigate(`/works/${restored}`)}
    />
  )
}
