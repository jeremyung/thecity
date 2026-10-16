//! Closed sets of entity kinds and lifecycle stages. Serialized as the same
//! strings the save format and snapshots used before they were enums.
use serde::{Deserialize, Serialize};

macro_rules! kind_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident = $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub enum $name {
            $(#[serde(rename = $text)] $variant),+
        }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            pub fn as_str(self) -> &'static str {
                match self {
                    $($name::$variant => $text),+
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

kind_enum!(
    /// Role of a network node. `Candidate` is a legacy-planner frontier proposal.
    NodeKind {
        Junction = "junction",
        Gateway = "gateway",
        Market = "market",
        Station = "station",
        RailGateway = "rail_gateway",
        RailTerminus = "rail_terminus",
        Elevated = "elevated",
        Candidate = "candidate",
    }
);

kind_enum!(
    /// Class of a road or track segment.
    RoadKind {
        Local = "local",
        Lane = "lane",
        Arterial = "arterial",
        Bridge = "bridge",
        Expressway = "expressway",
        Rail = "rail",
    }
);

kind_enum!(
    /// Construction and lifecycle stage shared by roads, buildings and projects.
    Stage {
        Survey = "survey",
        Clearing = "clearing",
        Foundation = "foundation",
        Structure = "structure",
        Surface = "surface",
        Finishing = "finishing",
        Platform = "platform",
        Open = "open",
        Renovation = "renovation",
        Demolition = "demolition",
        Demolished = "demolished",
        Done = "done",
    }
);

kind_enum!(
    /// Building use.
    BuildingKind {
        Home = "home",
        Apartment = "apartment",
        Shop = "shop",
        Cafe = "cafe",
        Office = "office",
    }
);

kind_enum!(
    /// What a construction project builds.
    ProjectKind {
        Road = "road",
        Expressway = "expressway",
        Rail = "rail",
        Station = "station",
        Building = "building",
        Extension = "extension",
        Redevelopment = "redevelopment",
        Completed = "completed",
    }
);

kind_enum!(
    /// Weather state.
    WeatherKind {
        Clear = "clear",
        Overcast = "overcast",
        Rain = "rain",
    }
);

kind_enum!(
    /// Event log entries shown to the viewer.
    EventKind {
        BuildingOpened = "BuildingOpened",
        BuildingRedeveloped = "BuildingRedeveloped",
        BusPassengerAlighted = "BusPassengerAlighted",
        BusPassengerBoarded = "BusPassengerBoarded",
        BusPassengerWaitingForRoute = "BusPassengerWaitingForRoute",
        BusServiceExpanded = "BusServiceExpanded",
        BusServiceStarted = "BusServiceStarted",
        BusStop = "BusStop",
        BusTripStarted = "BusTripStarted",
        BusinessClosed = "BusinessClosed",
        BusinessReopened = "BusinessReopened",
        DeliveryArrived = "DeliveryArrived",
        DeliveryDispatched = "DeliveryDispatched",
        DeliveryReturned = "DeliveryReturned",
        ExpresswayOpened = "ExpresswayOpened",
        FoundingShopOpened = "FoundingShopOpened",
        NeighborhoodSpineOpened = "NeighborhoodSpineOpened",
        ParkDedicated = "ParkDedicated",
        PlazaReserved = "PlazaReserved",
        RailOpened = "RailOpened",
        RailReturnQueued = "RailReturnQueued",
        RailTripStarted = "RailTripStarted",
        RailWorkStarted = "RailWorkStarted",
        RearExtensionCompleted = "RearExtensionCompleted",
        RearExtensionStarted = "RearExtensionStarted",
        RenovationStarted = "RenovationStarted",
        ResidentTripArrived = "ResidentTripArrived",
        ResidentTripStarted = "ResidentTripStarted",
        RoadOpened = "RoadOpened",
        ShrineReserved = "ShrineReserved",
        StationServiceStarted = "StationServiceStarted",
        TrainGatewayStop = "TrainGatewayStop",
        TrainPassengerAlighted = "TrainPassengerAlighted",
        TrainPassengerBoarded = "TrainPassengerBoarded",
        TrainPassengerWaitingForRoute = "TrainPassengerWaitingForRoute",
        TrainStationStop = "TrainStationStop",
        TransitAlightRouteResumed = "TransitAlightRouteResumed",
    }
);
