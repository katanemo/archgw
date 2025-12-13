export default function StudioLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="fixed inset-0 h-screen w-screen overflow-hidden">
      {children}
    </div>
  );
}
