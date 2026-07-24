use super::*;

impl StudioPlugin {
    pub(super) fn apply_asset_command(&mut self, engine: &mut Engine, command: StudioCommand) {
        match command {
            StudioCommand::RefreshAssets => match self.assets.refresh() {
                Ok(status) => { self.status = status.clone(); self.log(status); }
                Err(error) => self.log(error),
            },
            StudioCommand::ImportAssetFiles(paths) => match self.assets.import_files(&paths) {
                Ok((status, logs)) => {
                    self.status = status;
                    for log in logs { self.log(log); }
                }
                Err(error) => self.log(error),
            },
            StudioCommand::ReimportAsset(id) => match self.assets.reimport(id) {
                Ok(status) => self.status = status,
                Err(error) => self.log(error),
            },
            StudioCommand::ReimportAllAssets => match self.assets.reimport_all() {
                Ok(status) => self.status = status,
                Err(error) => self.log(error),
            },
            StudioCommand::ClearAssetCache => match self.assets.clear_cache() {
                Ok(status) => self.status = status,
                Err(error) => self.log(error),
            },
            StudioCommand::PruneAssetCache => match self.assets.prune_cache() {
                Ok(status) => self.status = status,
                Err(error) => self.log(error),
            },
            _ => unreachable!("non-asset command routed to asset handler"),
        }
    }
}
