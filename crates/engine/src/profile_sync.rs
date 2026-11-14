//! The user's embroidery profiles (machines, hoops of their own, My Machine, the last hoop) as one
//! value that can be handed to a store and taken back. Today the store is the local preferences
//! file ([`LocalProfiles`]); a later account store (add-on 38) implements [`ProfileSync`] and the
//! callers don't change. There is no sign-in or network code here.

use serde::{Deserialize, Serialize};
use sf_core::hoops::Hoop;
use sf_core::machines::MachineProfile;

use crate::Prefs;

/// Everything a user keeps about their machines and hoops. Names are the keys.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ProfileBundle {
    /// Machine profiles: the standard ones the user changed, and their own.
    pub machines: Vec<MachineProfile>,
    /// Hoops of the user's own.
    pub hoops: Vec<Hoop>,
    /// The machine chosen as My Machine, "" for none.
    pub my_machine: String,
    /// The hoop of the last design started, "" for none.
    pub last_hoop: String,
}

/// Where the profiles are kept. Implemented by the local preferences now, by an account later.
pub trait ProfileSync {
    /// The profiles as kept.
    fn pull(&self) -> Result<ProfileBundle, String>;
    /// Replace the profiles kept with `bundle`.
    fn push(&mut self, bundle: &ProfileBundle) -> Result<(), String>;
}

/// The local store: the profiles inside [`Prefs`], which the app saves with the preferences.
pub struct LocalProfiles<'a>(pub &'a mut Prefs);

impl ProfileSync for LocalProfiles<'_> {
    fn pull(&self) -> Result<ProfileBundle, String> {
        let p = &*self.0;
        Ok(ProfileBundle {
            machines: p.embroidery_machines.clone(),
            hoops: p.embroidery_custom_hoops.clone(),
            my_machine: p.my_machine.clone(),
            last_hoop: p.last_hoop.clone(),
        })
    }

    fn push(&mut self, bundle: &ProfileBundle) -> Result<(), String> {
        let p = &mut *self.0;
        p.embroidery_machines = bundle.machines.clone();
        p.embroidery_custom_hoops = bundle.hoops.clone();
        p.my_machine = bundle.my_machine.clone();
        p.last_hoop = bundle.last_hoop.clone();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Machines and hoops survive a trip through the bundle and through the preferences file.
    #[test]
    fn profiles_round_trip_through_the_bundle_and_the_preferences_file() {
        let mut s = crate::Session::new();
        s.execute("embroidery.machine.save", &json!({"name": "Shop", "format": "dst", "speed": 700.0})).unwrap();
        s.execute("embroidery.hoop.saveCustom", &json!({"name": "Hat", "width": 200, "height": 60})).unwrap();
        let bundle = LocalProfiles(&mut s.prefs).pull().unwrap();
        assert_eq!((bundle.machines.len(), bundle.hoops.len()), (1, 1));
        // the bundle is plain JSON, for any store
        let text = serde_json::to_string(&bundle).unwrap();
        assert_eq!(serde_json::from_str::<ProfileBundle>(&text).unwrap(), bundle);
        // a reload: the preferences written and read again
        let reloaded: Prefs = serde_json::from_str(&serde_json::to_string(&s.prefs).unwrap()).unwrap();
        let mut fresh = crate::Session::new();
        fresh.prefs = reloaded;
        let list = fresh.execute("embroidery.machine.list", &json!({})).unwrap();
        assert!(list["machines"].as_array().unwrap().iter().any(|m| m["name"] == "Shop" && m["speed"] == 700.0));
        assert_eq!(fresh.execute("embroidery.myMachine", &json!({})).unwrap()["own"][0]["name"], "Hat");
        // another store's bundle replaces the local one
        let mut other = crate::Session::new();
        LocalProfiles(&mut other.prefs).push(&bundle).unwrap();
        assert_eq!(LocalProfiles(&mut other.prefs).pull().unwrap(), bundle);
        assert_eq!(ProfileBundle::default(), serde_json::from_str("{}").unwrap());
    }
}
