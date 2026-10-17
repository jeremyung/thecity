//! Closed sets of entity kinds and lifecycle stages. Serialized as the same
//! strings the save format and snapshots used before they were enums.
use serde::{Deserialize, Serialize};

macro_rules! kind_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident = $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ts_rs::TS)]
        #[ts(export)]
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
        Arterial = "arterial",
        Local = "local",
        Lane = "lane",
        Bridge = "bridge",
        Rail = "rail",
        Expressway = "expressway",
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

kind_enum!(
    /// What a visible traveler is doing.
    TravelerActivity {
        Commute = "commute",
        Delivery = "delivery",
        DeliveryReturn = "delivery_return",
        NeighborhoodErrandOutbound = "neighborhood_errand_outbound",
        NeighborhoodErrandReturning = "neighborhood_errand_returning",
        NightShift = "night_shift",
        Return = "return",
        Stroll = "stroll",
        WalkingHome = "walking_home",
        WalkingToBus = "walking_to_bus",
        WalkingToStation = "walking_to_station",
    }
);

kind_enum!(
    /// What a named resident is doing.
    ResidentActivity {
        AtHome = "at_home",
        AtDestination = "at_destination",
        Commuting = "commuting",
        ReturningHome = "returning_home",
        Strolling = "strolling",
        AboardBus = "aboard_bus",
        RidingTrain = "riding_train",
        WaitingForBus = "waiting_for_bus",
        WaitingForTrain = "waiting_for_train",
        WaitingForRoute = "waiting_for_route",
        WalkingHome = "walking_home",
        WalkingHomeFromBus = "walking_home_from_bus",
        WalkingToBus = "walking_to_bus",
        WalkingToStation = "walking_to_station",
        WalkingToWorkFromBus = "walking_to_work_from_bus",
        WorkingExternal = "working_external",
    }
);

kind_enum!(
    /// A resident's daily pattern.
    ResidentRoutine {
        Neighbor = "neighbor",
        Worker = "worker",
        RailCommuter = "rail_commuter",
    }
);

kind_enum!(
    /// Where a resident is in a multi-leg transit trip; `None` when not travelling.
    Transfer {
        None = "",
        Home = "home",
        RidingBus = "riding_bus",
        RidingInbound = "riding_inbound",
        RidingOutbound = "riding_outbound",
        WaitingAlight = "waiting_alight",
        WaitingBus = "waiting_bus",
        WaitingGateway = "waiting_gateway",
        WaitingStation = "waiting_station",
        WalkingFromBus = "walking_from_bus",
        WalkingHome = "walking_home",
        WalkingToBus = "walking_to_bus",
        WalkingToStation = "walking_to_station",
        WorkingExternal = "working_external",
    }
);

impl Default for Transfer {
    fn default() -> Self {
        Transfer::None
    }
}

impl TravelerActivity {
    /// Outbound or returning leg of an aggregate neighborhood errand.
    pub fn is_errand(self) -> bool {
        matches!(
            self,
            TravelerActivity::NeighborhoodErrandOutbound
                | TravelerActivity::NeighborhoodErrandReturning
        )
    }

    /// A delivery van or its return trip.
    pub fn is_delivery(self) -> bool {
        matches!(
            self,
            TravelerActivity::Delivery | TravelerActivity::DeliveryReturn
        )
    }
}
