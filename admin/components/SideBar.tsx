import Link from 'next/link';
import { useRouter } from 'next/router';

const SideBar = () => {
  const router = useRouter();
  return (
    <div className="sidebar">
      <div className="sidebar__header">Rust multiplayer</div>
      <nav className="sidebar__nav">
        <Link href="/">
          <a
            className="sidebar__link"
            data-active={router.pathname === '/' ? '' : null}
          >
            Connections
          </a>
        </Link>
      </nav>
    </div>
  );
};

export default SideBar;
