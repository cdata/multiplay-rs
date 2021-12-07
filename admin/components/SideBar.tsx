import Link from 'next/link';

const SideBar = () => {
  return (
    <div className="sidebar">
      <div className="sidebar__header">Rust multiplayer</div>
      <nav className="sidebar__nav">
        <Link href="/">Connections</Link>
      </nav>
    </div>
  );
};

export default SideBar;
