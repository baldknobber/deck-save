export default function SyncWizard() {
  return (
    <div className="max-w-2xl">
      <h2 className="text-2xl font-bold mb-6">Sync Setup</h2>
      <div className="bg-gray-800 rounded-lg p-8 border border-gray-700 text-center">
        <p className="text-gray-400 mb-4">
          Sync your backups between devices using Syncthing.
        </p>
        <p className="text-sm text-gray-500">
          The sync wizard will guide you through pairing your Steam Deck and PC.
        </p>
      </div>
    </div>
  );
}
