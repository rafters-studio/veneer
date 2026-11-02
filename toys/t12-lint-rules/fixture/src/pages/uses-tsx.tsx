const dynamic = "p-[9px]";

export function Page() {
  return (
    <main>
      {/* T1 flag */}
      <div className="p-[32px]" />
      {/* T2 flag: expression literal */}
      <div className={"w-[10px]"} />
      {/* T3 flag: template with no substitution */}
      <div className={`h-[3px]`} />
      {/* T4 ok: modifier */}
      <div className="group-data-[state=open]:p-4" />
      {/* T5 dynamic: nothing */}
      <div className={dynamic} />
    </main>
  );
}
