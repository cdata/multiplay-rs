import SideBar from './SideBar';

type LayoutProps = {
  children: React.ReactNode;
};

export default function Layout({ children }: LayoutProps) {
  return (
    <div className="layout">
      <SideBar />
      <main className="flow">{children}</main>
    </div>
  );
}
