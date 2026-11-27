use wasapi::{DeviceEnumerator, Direction, SessionState};

/// About -60 dBFS: quieter sessions count as silent.
const AUDIBLE_PEAK: f32 = 0.001;

/// Ids of the processes whose audio sessions are playing sound right now, on
/// any active output device.
///
/// Initializes COM for the calling thread in the multithreaded apartment, so
/// it must not run on a UI thread.
pub fn audible_processes() -> Result<Vec<u32>, crate::Error> {
    wasapi::initialize_mta().ok()?;

    let devices = DeviceEnumerator::new()?.get_device_collection(Direction::Render)?;

    let mut audible = Vec::new();
    for device in &devices {
        let sessions = device?
            .get_iaudiosessionmanager()?
            .get_audiosessionenumerator()?;

        for index in 0..sessions.get_count()? {
            let session = sessions.get_session(index)?;
            if session.get_state()? != SessionState::Active {
                continue;
            }

            let peak = session.get_audiometerinformation()?.get_peak_value()?;
            if peak > AUDIBLE_PEAK {
                audible.push(session.get_process_id()?);
            }
        }
    }

    Ok(audible)
}
