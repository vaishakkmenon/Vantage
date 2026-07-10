export function LoadingScreen() {
    return (
        <div className="flex h-screen items-center justify-center bg-background text-foreground">
            <p className="text-sm text-muted-foreground">Loading chess engine…</p>
        </div>
    );
}