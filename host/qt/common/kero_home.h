#pragma once

#include <QString>

/** Returns the platform-owned default location used to find the KERO-home locator. */
QString defaultKeroHomeLocation();

/** Resolves KERO_HOME or the direct user-owned default. */
QString configuredKeroHome();

/**
 * Reports the result of validating or provisioning a KERO global home.
 *
 * A valid home contains the global configuration, local-data, and mount roots.
 */
struct KeroHomeResult {
    bool ready = false;
    QString path;
    QString error;
};

/**
 * Resolves the configured KERO home and provisions its required layout.
 *
 * @return the resolved home path and either a ready state or a diagnostic.
 */
KeroHomeResult ensureKeroHome();
